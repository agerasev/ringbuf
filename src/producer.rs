use std::mem::{self, MaybeUninit};
use std::sync::{Arc, atomic::{Ordering}};
//use std::io::{self, Read, Write};

use crate::ring_buffer::*;
//use crate::consumer::Consumer;


/// Producer part of ring buffer.
pub struct Producer<T> {
    pub(crate) rb: Arc<RingBuffer<T>>,
}


impl<T: Sized> Producer<T> {
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

    /// The length of the data in the buffer.
    pub fn len(&self) -> usize {
        self.rb.len()
    }

    /// The remaining space in the buffer.
    pub fn remaining(&self) -> usize {
        self.rb.remaining()
    }

    /// Allows to write into ring buffer memory directry.
    ///
    /// *This function is unsafe because it gives access to possibly uninitialized memory*
    ///
    /// The method takes a function `f` as argument.
    /// `f` takes two slices of ring buffer content (the second one of both of them may be empty).
    /// First slice contains older elements.
    ///
    /// `f` should return number of elements been written.
    /// There is no checks for returned number - it remains on the developer's conscience.
    ///
    /// The method *always* calls `f` even if ring buffer is full.
    ///
    /// The method returns number returned from `f`.
    pub unsafe fn push_access<F: FnOnce(&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) -> usize>(&mut self, f: F) -> usize {
        let head = self.rb.head.load(Ordering::Acquire);
        let tail = self.rb.tail.load(Ordering::Acquire);
        let len = self.rb.data.get_ref().len();

        let ranges = if tail >= head {
            if head > 0 {
                (tail..len, 0..(head - 1))
            } else {
                if tail < len - 1 {
                    (tail..(len - 1), 0..0)
                } else {
                    (0..0, 0..0)
                }
            }
        } else {
            if tail < head - 1 {
                (tail..(head - 1), 0..0)
            } else {
                (0..0, 0..0)
            }
        };

        let slices = (
            &mut self.rb.data.get_mut()[ranges.0],
            &mut self.rb.data.get_mut()[ranges.1],
        );

        let n = f(slices.0, slices.1);

        if n > 0 {
            let new_tail = (tail + n) % len;
            self.rb.tail.store(new_tail, Ordering::Release);
        }
        n
    }

    /// Appends an element to the ring buffer.
    /// On failure returns an error containing the element that hasn't beed appended.
    pub fn push(&mut self, elem: T) -> Result<(), T> {
        let mut elem_mu = MaybeUninit::new(elem);
        let n = unsafe { self.push_access(|slice, _| {
            if slice.len() > 0 {
                mem::swap(slice.get_unchecked_mut(0), &mut elem_mu);
                1
            } else {
                0
            }
        }) };
        match n {
            0 => Err(unsafe { elem_mu.assume_init() }),
            1 => Ok(()),
            _ => unreachable!(),
        }
    }

    pub fn push_fn<F: FnMut() -> Option<T>>(&mut self, mut f: F) -> usize {
        let af = |left: &mut [MaybeUninit<T>], right: &mut [MaybeUninit<T>]| {
            for (i, dst) in left.iter_mut().enumerate() {
                match f() {
                    Some(e) => mem::replace(dst, MaybeUninit::new(e)),
                    None => return i,
                };
            }
            for (i, dst) in right.iter_mut().enumerate() {
                match f() {
                    Some(e) => mem::replace(dst, MaybeUninit::new(e)),
                    None => return i + left.len(),
                };
            }
            left.len() + right.len()
        };
        unsafe { self.push_access(af) }
    }

    /// Appends elements from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    ///
    /// Returns count of elements been appended to the ring buffer.
    pub fn push_iter<I: Iterator<Item=T>>(&mut self, elems: &mut I) -> usize {
        self.push_fn(|| elems.next())
    }
}

impl<T: Sized + Copy> Producer<T> {
    /// Appends elements from slice to the ring buffer.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// Returns count of elements been appended to the ring buffer.
    pub fn push_slice(&mut self, elems: &[T]) -> usize {
        self.push_iter(&mut elems.iter().map(|e| *e))
    }
}
/*
impl<T: Sized> Producer<T> {
    /// Removes at most `count` elements from the `Consumer` of the ring buffer
    /// and appends them to the `Producer` of the another one.
    /// If `count` is `None` then as much as possible elements will be moved.
    ///
    /// On success returns count of elements been moved.
    pub fn move_from(&mut self, other: &mut Consumer<T>, count: Option<usize>)
    -> Result<usize, MoveSliceError> {
        let move_fn = |left: &mut [MaybeUninit<T>], right: &mut [MaybeUninit<T>]|
        -> Result<(usize, ()), PopSliceError> {
            let (left, right) = match count {
                Some(c) => {
                    if c < left.len() {
                        (&mut left[0..c], &mut right[0..0])
                    } else if c < left.len() + right.len() {
                        let l = c - left.len();
                        (left, &mut right[0..l])
                    } else {
                        (left, right)
                    }
                },
                None => (left, right)
            };
            other.pop_slice(left).and_then(|n| {
                if n == left.len() {
                    other.pop_slice(right).and_then(|m| {
                        Ok((n + m, ()))
                    }).or_else(|e| {
                        match e {
                            PopSliceError::Empty => Ok((n, ())),
                        }
                    })
                } else {
                    debug_assert!(n < left.len());
                    Ok((n, ()))
                }
            })
        };
        match unsafe { self.push_access(move_fn) } {
            Ok(res) => match res {
                Ok((n, ())) => Ok(n),
                Err(e) => match e {
                    PopSliceError::Empty => Err(MoveSliceError::Empty),
                },
            },
            Err(e) => match e {
                PushAccessError::Full => Err(MoveSliceError::Full),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}
*/
/*
impl Producer<u8> {
    /// Reads at most `count` bytes
    /// from [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) instance
    /// and appends them to the ring buffer.
    /// If `count` is `None` then as much as possible bytes will be read.
    pub fn read_from(&mut self, reader: &mut dyn Read, count: Option<usize>)
    -> Result<usize, ReadFromError> {
        let push_fn = |left: &mut [u8], _right: &mut [u8]|
        -> Result<(usize, ()), io::Error> {
            let left = match count {
                Some(c) => {
                    if c < left.len() {
                        &mut left[0..c]
                    } else {
                        left
                    }
                },
                None => left,
            };
            reader.read(left).and_then(|n| {
                if n <= left.len() {
                    Ok((n, ()))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Read operation returned invalid number",
                    ))
                }
            })
        };
        match unsafe { self.push_access(push_fn) } {
            Ok(res) => match res {
                Ok((n, ())) => Ok(n),
                Err(e) => Err(ReadFromError::Read(e)),
            },
            Err(e) => match e {
                PushAccessError::Full => Err(ReadFromError::RbFull),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl Write for Producer<u8> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.push_slice(buffer).or_else(|e| match e {
            PushSliceError::Full => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Ring buffer is full",
            ))
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
*/
