use alloc::sync::Arc;
use core::{
    cmp::{self, min},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::{Deref, Range},
    ptr::copy_nonoverlapping,
    slice,
    sync::atomic,
};
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

use crate::ring_buffer::RingBufferHead;

pub trait RbHeadRef<T> {
    type Target: RingBufferHead<T>;
    fn rb(&self) -> &Self::Target;
}

impl<T, B: RingBufferHead<T>> RbHeadRef<T> for Arc<B> {
    type Target = B;
    fn rb(&self) -> &B {
        self.as_ref()
    }
}

impl<'a, T, B: RingBufferHead<T>> RbHeadRef<T> for &'a B {
    type Target = B;
    fn rb(&self) -> &B {
        *self
    }
}

/// Consumer part of ring buffer.
pub struct Consumer<T, B: RingBufferHead<T>, R: RbHeadRef<T, Target = B>> {
    rb_ref: R,
    _phantom: PhantomData<(T, B)>,
}

impl<T, B: RingBufferHead<T>, R: RbHeadRef<T, Target = B>> Consumer<T, B, R> {
    pub(crate) fn new(rb_ref: R) -> Self {
        Self {
            rb_ref,
            _phantom: PhantomData,
        }
    }
}

impl<T, B: RingBufferHead<T>, R: RbHeadRef<T, Target = B>> Deref for Consumer<T, B, R> {
    type Target = B;
    fn deref(&self) -> &B {
        self.rb_ref.rb()
    }
}

impl<T, B: RingBufferHead<T>, R: RbHeadRef<T, Target = B>> Consumer<T, B, R> {
    /// Returns a pair of slices which contain, in order, the contents of the `RingBuffer`.
    ///
    /// *The slices may not include elements pushed to the buffer by concurring producer after the method call.*
    fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let (left, right) = self.occupied_slices();
            // TODO: Change to `slice_assume_init` on `maybe_uninit_slice` stabilization.
            (
                &*(left as *const [MaybeUninit<T>] as *const [T]),
                &*(right as *const [MaybeUninit<T>] as *const [T]),
            )
        }
    }

    /// Returns a pair of slices which contain, in order, the contents of the `RingBuffer`.
    ///
    /// *The slices may not include elements pushed to the buffer by concurring producer after the method call.*
    fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            let (left, right) = self.occupied_slices();
            // TODO: Change to `slice_assume_init_mut` on `maybe_uninit_slice` stabilization.
            (
                &mut *(left as *mut [MaybeUninit<T>] as *mut [T]),
                &mut *(right as *mut [MaybeUninit<T>] as *mut [T]),
            )
        }
    }

    /// Removes latest element from the ring buffer and returns it.
    /// Returns `None` if the ring buffer is empty.
    fn pop(&mut self) -> Option<T> {
        unsafe {
            let (left, _) = self.occupied_slices();
            if let Some(elem) = left.iter().next() {
                let ret = elem.as_ptr().read();
                self.move_head(1);
                Some(ret)
            } else {
                None
            }
        }
    }

    /*
    /// Returns a front-to-back iterator.
    fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        let (left, right) = self.as_slices();

        left.iter().chain(right.iter())
    }

    /// Returns a front-to-back iterator that returns mutable references.
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        let (left, right) = self.as_mut_slices();

        left.iter_mut().chain(right.iter_mut())
    }

    /// Removes at most `n` and at least `min(n, Consumer::len())` items from the buffer and safely drops them.
    ///
    /// If there is no concurring producer activity then exactly `min(n, Consumer::len())` items are removed.
    ///
    /// Returns the number of deleted items.
    ///
    ///
    /// ```rust
    /// # extern crate ringbuf;
    /// # use ringbuf::RingBuffer;
    /// # fn main() {
    /// let rb = RingBuffer::<i32>::new(8);
    /// let (mut prod, mut cons) = rb.split();
    ///
    /// assert_eq!(prod.push_iter(&mut (0..8)), 8);
    ///
    /// assert_eq!(cons.discard(4), 4);
    /// assert_eq!(cons.discard(8), 4);
    /// assert_eq!(cons.discard(8), 0);
    /// # }
    /// ```
    fn discard(&mut self, n: usize) -> usize {
        unsafe {
            self.pop_access(|left, right| {
                let (mut cnt, mut rem) = (0, n);
                let left_elems = if rem <= left.len() {
                    cnt += rem;
                    left.get_unchecked_mut(0..rem)
                } else {
                    cnt += left.len();
                    left
                };
                rem = n - cnt;

                let right_elems = if rem <= right.len() {
                    cnt += rem;
                    right.get_unchecked_mut(0..rem)
                } else {
                    cnt += right.len();
                    right
                };

                for e in left_elems.iter_mut().chain(right_elems.iter_mut()) {
                    e.as_mut_ptr().drop_in_place();
                }

                cnt
            })
        }
    }

    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns count of elements been moved.
    fn move_to(&mut self, other: &mut Producer<T>, count: Option<usize>) -> usize {
        move_items(self, other, count)
    }
    */
}

/*
impl<T: Sized> Iterator for Consumer<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.pop()
    }
}

impl<T: Sized + Copy> Consumer<T> {
    /// Removes first elements from the ring buffer and writes them into a slice.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been removed from the ring buffer.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> usize {
        unsafe { self.pop_copy(&mut *(elems as *mut [T] as *mut [MaybeUninit<T>])) }
    }
}

#[cfg(feature = "std")]
impl Consumer<u8> {
    /// Removes at most first `count` bytes from the ring buffer and writes them into
    /// a [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html) instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    ///
    /// Returns `Ok(n)` if `write` succeeded. `n` is number of bytes been written.
    /// `n == 0` means that either `write` returned zero or ring buffer is empty.
    ///
    /// If `write` is failed or returned an invalid number then error is returned.
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
                    .write(&*(left as *const [MaybeUninit<u8>] as *const [u8]))
                    .and_then(|n| {
                        if n <= left.len() {
                            Ok(n)
                        } else {
                            Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "Write operation returned an invalid number",
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

#[cfg(feature = "std")]
impl Read for Consumer<u8> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.pop_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }
}
*/
