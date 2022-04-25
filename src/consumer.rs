use alloc::sync::Arc;
use core::{
    cmp::{self, min},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::{Deref, Range},
    ptr::{self, copy_nonoverlapping},
    slice,
    sync::atomic,
};
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

use crate::ring_buffer::{Container, RingBuffer, RingBufferRef};

/// Consumer part of ring buffer.
pub struct Consumer<T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    rb: R,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C, R> Consumer<T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    pub(crate) fn new(rb: R) -> Self {
        Self {
            rb,
            _phantom: PhantomData,
        }
    }
}

impl<T, C, R> Consumer<T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    pub fn capacity(&self) -> usize {
        self.rb.capacity()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// The result is relevant until you push items to the producer.
    pub fn is_empty(&self) -> bool {
        self.rb.is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring activity of the consumer.*
    pub fn is_full(&self) -> bool {
        self.rb.is_full()
    }

    /// The length of the data stored in the buffer.
    ///
    /// Actual length may be equal to or less than the returned value.
    pub fn len(&self) -> usize {
        self.rb.occupied_len()
    }

    /// The remaining space in the buffer.
    ///
    /// Actual remaining space may be equal to or greater than the returning value.
    pub fn remaining(&self) -> usize {
        self.rb.vacant_len()
    }

    /// Returns a pair of slices which contain, in order, the contents of the `RingBuffer`.
    ///
    /// *The slices may not include elements pushed to the buffer by concurring producer after the method call.*
    pub fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let (left, right) = self.rb.occupied_slices();
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
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            let (left, right) = self.rb.occupied_slices();
            // TODO: Change to `slice_assume_init_mut` on `maybe_uninit_slice` stabilization.
            (
                &mut *(left as *mut [MaybeUninit<T>] as *mut [T]),
                &mut *(right as *mut [MaybeUninit<T>] as *mut [T]),
            )
        }
    }

    /// Removes latest element from the ring buffer and returns it.
    /// Returns `None` if the ring buffer is empty.
    pub fn pop(&mut self) -> Option<T> {
        let (left, _) = unsafe { self.rb.occupied_slices() };
        match left.iter().next() {
            Some(place) => {
                let elem = unsafe { place.as_ptr().read() };
                unsafe { self.rb.move_head(1) };
                Some(elem)
            }
            None => None,
        }
    }

    /// Returns iterator that removes elements one by one from the ring buffer.
    pub fn pop_iter(&mut self) -> PopIterator<'_, T, C, R> {
        PopIterator { consumer: self }
    }

    /// Returns a front-to-back iterator.
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        let (left, right) = self.as_slices();

        left.iter().chain(right.iter())
    }

    /// Returns a front-to-back iterator that returns mutable references.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
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
    pub fn skip(&mut self, count: usize) -> usize {
        let actual_count = cmp::min(count, self.rb.occupied_len());
        self.rb.skip(actual_count);
        actual_count
    }

    /*
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

pub struct PopIterator<'a, T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    consumer: &'a mut Consumer<T, C, R>,
}

impl<'a, T, C, R> Iterator for PopIterator<'a, T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.consumer.pop()
    }
}

/*
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
