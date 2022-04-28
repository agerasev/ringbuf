use core::{
    cmp,
    marker::PhantomData,
    mem::{self, MaybeUninit},
};
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

use crate::{
    producer::Producer,
    ring_buffer::{move_items, Container, RingBufferRef},
};

/// Consumer part of ring buffer.
pub struct Consumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    rb: R,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C, R> Consumer<T, C, R>
where
    C: Container<T>,
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
    C: Container<T>,
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

    pub unsafe fn as_uninit_slices(&self) -> (&[MaybeUninit<T>], &[MaybeUninit<T>]) {
        let (left, right) = self.rb.occupied_slices();
        (left, right)
    }
    pub unsafe fn as_mut_uninit_slices(
        &mut self,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.rb.occupied_slices()
    }

    pub unsafe fn accept(&mut self, count: usize) {
        self.rb.shift_head(count);
    }

    /// Returns a pair of slices which contain, in order, the contents of the `RingBuffer`.
    ///
    /// *The slices may not include elements pushed to the buffer by concurring producer after the method call.*
    pub fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let (left, right) = self.as_uninit_slices();
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
            let (left, right) = self.as_mut_uninit_slices();
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
        let (left, _) = unsafe { self.as_uninit_slices() };
        match left.iter().next() {
            Some(place) => {
                let elem = unsafe { place.as_ptr().read() };
                unsafe { self.accept(1) };
                Some(elem)
            }
            None => None,
        }
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

    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns count of elements been moved.
    pub fn move_to<Cd, Rd>(
        &mut self,
        other: &mut Producer<T, Cd, Rd>,
        count: Option<usize>,
    ) -> usize
    where
        Cd: Container<T>,
        Rd: RingBufferRef<T, Cd>,
    {
        move_items(self, other, count)
    }
}

impl<T, C, R> Iterator for Consumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.pop()
    }
}

impl<T: Copy, C, R> Consumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    /// Removes first elements from the ring buffer and writes them into a slice.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been removed from the ring buffer.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> usize {
        let (left, right) = unsafe { self.as_uninit_slices() };
        // TODO: Change to `write_slice` on `maybe_uninit_write_slice` stabilization.
        let elems: &mut [MaybeUninit<T>] = unsafe { mem::transmute(elems) };
        let count = if elems.len() < left.len() {
            elems.copy_from_slice(&left[..elems.len()]);
            elems.len()
        } else {
            let (left_elems, elems) = elems.split_at_mut(left.len());
            left_elems.copy_from_slice(left);
            left.len()
                + if elems.len() < right.len() {
                    elems.copy_from_slice(&right[..elems.len()]);
                    elems.len()
                } else {
                    elems[..right.len()].copy_from_slice(right);
                    right.len()
                }
        };
        unsafe { self.accept(count) };
        count
    }
}

#[cfg(feature = "std")]
impl<C, R> Consumer<u8, C, R>
where
    C: Container<u8>,
    R: RingBufferRef<u8, C>,
{
    /// Removes at most first `count` bytes from the ring buffer and writes them into
    /// a [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html) instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    ///
    /// Returns `Ok(n)` if `write` succeeded. `n` is number of bytes been written.
    /// `n == 0` means that either `write` returned zero or ring buffer is empty.
    ///
    /// If `write` is failed or returned an invalid number then error is returned.
    pub fn write_into<S: Write>(
        &mut self,
        writer: &mut S,
        count: Option<usize>,
    ) -> io::Result<usize> {
        let (left, _) = unsafe { self.as_uninit_slices() };
        let count = cmp::min(count.unwrap_or(left.len()), left.len());
        let left = &left[..count];
        writer
            .write(unsafe { &*(left as *const [MaybeUninit<u8>] as *const [u8]) })
            .map(|n| {
                assert!(n <= left.len());
                n
            })
    }
}

#[cfg(feature = "std")]
impl<C, R> Read for Consumer<u8, C, R>
where
    C: Container<u8>,
    R: RingBufferRef<u8, C>,
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.pop_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }
}
