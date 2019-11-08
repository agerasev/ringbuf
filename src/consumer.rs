use std::{
    cmp::min,
    io::{self, Read, Write},
    mem::{self, transmute, MaybeUninit},
    ptr::copy_nonoverlapping,
    sync::{atomic::Ordering, Arc},
};

use crate::{producer::Producer, ring_buffer::*};

/// Consumer part of ring buffer.
pub struct Consumer<T> {
    pub(crate) rb: Arc<RingBuffer<T>>,
}

impl<T: Sized> Consumer<T> {
    /// Returns capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        self.rb.capacity()
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.rb.is_empty()
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        self.rb.is_full()
    }

    /// The length of the data in the buffer
    pub fn len(&self) -> usize {
        self.rb.len()
    }

    /// The remaining space in the buffer.
    pub fn remaining(&self) -> usize {
        self.rb.remaining()
    }

    /// Allows to read from ring buffer memory directry.
    ///
    /// *This function is unsafe because it gives access to possibly uninitialized memory*
    ///
    /// The method takes a function `f` as argument.
    /// `f` takes two slices of ring buffer content (the second one of both of them may be empty).
    /// First slice contains older elements.
    ///
    /// `f` should return number of elements been read.
    /// There is no checks for returned number - it remains on the developer's conscience.
    ///
    /// The method *always* calls `f` even if ring buffer is empty.
    ///
    /// The method returns number returned from `f`.
    pub unsafe fn pop_access<F>(&mut self, f: F) -> usize
    where
        F: FnOnce(&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) -> usize,
    {
        let head = self.rb.head.load(Ordering::Acquire);
        let tail = self.rb.tail.load(Ordering::Acquire);
        let len = self.rb.data.get_ref().len();

        let ranges = if head < tail {
            (head..tail, 0..0)
        } else if head > tail {
            (head..len, 0..tail)
        } else {
            (0..0, 0..0)
        };

        let slices = (
            &mut self.rb.data.get_mut()[ranges.0],
            &mut self.rb.data.get_mut()[ranges.1],
        );

        let n = f(slices.0, slices.1);

        if n > 0 {
            let new_head = (head + n) % len;
            self.rb.head.store(new_head, Ordering::Release);
        }
        n
    }

    pub unsafe fn pop_copy(&mut self, elems: &mut [MaybeUninit<T>]) -> usize {
        self.pop_access(|left, right| {
            if elems.len() < left.len() {
                copy_nonoverlapping(left.as_ptr(), elems.as_mut_ptr(), elems.len());
                elems.len()
            } else {
                copy_nonoverlapping(left.as_ptr(), elems.as_mut_ptr(), left.len());
                if elems.len() < left.len() + right.len() {
                    copy_nonoverlapping(
                        right.as_ptr(),
                        elems.as_mut_ptr().offset(left.len() as isize),
                        elems.len() - left.len(),
                    );
                    elems.len()
                } else {
                    copy_nonoverlapping(
                        right.as_ptr(),
                        elems.as_mut_ptr().offset(left.len() as isize),
                        right.len(),
                    );
                    left.len() + right.len()
                }
            }
        })
    }

    /// Removes first element from the ring buffer and returns it.
    pub fn pop(&mut self) -> Option<T> {
        let mut elem_mu = MaybeUninit::uninit();
        let n = unsafe {
            self.pop_access(|slice, _| {
                if slice.len() > 0 {
                    mem::swap(slice.get_unchecked_mut(0), &mut elem_mu);
                    1
                } else {
                    0
                }
            })
        };
        match n {
            0 => None,
            1 => Some(unsafe { elem_mu.assume_init() }),
            _ => unreachable!(),
        }
    }

    pub fn pop_fn<F: FnMut(T) -> bool>(&mut self, mut f: F, count: Option<usize>) -> usize {
        unsafe {
            self.pop_access(|left, right| {
                let lb = match count {
                    Some(n) => min(n, left.len()),
                    None => left.len(),
                };
                for (i, dst) in left[0..lb].iter_mut().enumerate() {
                    if !f(mem::replace(dst, MaybeUninit::uninit()).assume_init()) {
                        return i;
                    }
                }
                if lb < left.len() {
                    return lb;
                }

                let rb = match count {
                    Some(n) => min(n - lb, right.len()),
                    None => right.len(),
                };
                for (i, dst) in right[0..rb].iter_mut().enumerate() {
                    if !f(mem::replace(dst, MaybeUninit::uninit()).assume_init()) {
                        return i + lb;
                    }
                }
                left.len() + right.len()
            })
        }
    }

    /// Removes at most `count` elements from the `Consumer` of the ring buffer
    /// and appends them to the `Producer` of the another one.
    /// If `count` is `None` then as much as possible elements will be moved.
    ///
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been moved.
    pub fn move_to(&mut self, other: &mut Producer<T>, count: Option<usize>) -> usize {
        move_items(self, other, count)
    }
}

impl<T: Sized + Copy> Consumer<T> {
    /// Removes first elements from the ring buffer and writes them into a slice.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been removed from the ring buffer.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> usize {
        unsafe { self.pop_copy(transmute::<&mut [T], &mut [MaybeUninit<T>]>(elems)) }
    }
}

impl Consumer<u8> {
    /// Removes at most first `count` bytes from the ring buffer and writes them into
    /// a [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html) instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    pub fn write_into(
        &mut self,
        writer: &mut dyn Write,
        count: Option<usize>,
    ) -> io::Result<usize> {
        let mut err = None;
        let n = unsafe {
            self.pop_access(|left, _| -> usize {
                let left = match count {
                    Some(c) => {
                        if c < left.len() {
                            &mut left[0..c]
                        } else {
                            left
                        }
                    }
                    None => left,
                };
                match writer
                    .write(transmute::<&mut [MaybeUninit<u8>], &mut [u8]>(left))
                    .and_then(|n| {
                        if n <= left.len() {
                            Ok(n)
                        } else {
                            Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "Write operation returned invalid number",
                            ))
                        }
                    }) {
                    Ok(n) => n,
                    Err(e) => {
                        err = Some(e);
                        0
                    }
                }
            })
        };
        match err {
            Some(e) => Err(e),
            None => Ok(n),
        }
    }
}

impl Read for Consumer<u8> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.pop_slice(buffer);
        if n == 0 && buffer.len() != 0 {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Ring buffer is empty",
            ))
        } else {
            Ok(n)
        }
    }
}
