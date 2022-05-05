use crate::{
    counter::{Counter, LocalHeadCounter},
    producer::LocalProducer,
    ring_buffer::{Container, Storage},
    transfer::transfer_local,
    utils::{slice_assume_init_mut, slice_assume_init_ref, write_uninit_slice},
};
use core::{cmp, mem::MaybeUninit, ptr, slice};

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Consumer part of ring buffer.
///
/// Generic over item type, ring buffer container and ring buffer reference.
pub struct LocalConsumer<'a, T, C: Container<T>> {
    storage: &'a Storage<T, C>,
    counter: LocalHeadCounter<'a>,
}

impl<'a, T, C: Container<T>> LocalConsumer<'a, T, C> {
    pub(crate) fn new(storage: &'a Storage<T, C>, counter: LocalHeadCounter<'a>) -> Self {
        Self { storage, counter }
    }

    unsafe fn read(&self) -> T {
        debug_assert!(!self.counter.is_empty());
        self.storage
            .as_slice()
            .get_unchecked(self.counter.head() % self.counter.len())
            .assume_init_read()
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.counter.is_empty()
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        self.counter.is_full()
    }

    /// The number of elements stored in the buffer.
    pub fn len(&self) -> usize {
        self.counter.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    pub fn remaining(&self) -> usize {
        self.counter.vacant_len()
    }

    /// Provides a direct access to the ring buffer occupied memory.
    /// The difference from `Self::as_slices` is that this method provides slices of `MaybeUninit<T>`, so elements may be moved out of slices.  
    ///
    /// Returns a pair of slices of stored elements, the second one may be empty.
    /// Elements with lower indices in slice are older. First slice contains older elements that second one.
    ///
    /// # Safety
    ///
    /// All elements are initialized. Elements must be removed starting from the beginning of first slice.
    /// When all elements are removed from the first slice then elements must be removed from the beginning of the second slice.
    ///
    /// *This method must be followed by `Self::advance` call with the number of elements being removed previously as argument.*
    /// *No other mutating calls allowed before that.*
    pub unsafe fn as_uninit_slices(&self) -> (&[MaybeUninit<T>], &[MaybeUninit<T>]) {
        let ranges = self.counter.occupied_ranges();
        let ptr = self.storage.as_slice().as_ptr();
        (
            slice::from_raw_parts(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }
    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// See `as_uninit_slices` for details.
    ///
    /// # Safety
    ///
    /// The same as for `as_uninit_slices` except that elements could be modified without being removed.
    pub unsafe fn as_mut_uninit_slices(
        &mut self,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let ranges = self.counter.occupied_ranges();
        let ptr = self.storage.as_mut_slice().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// Moves `head` counter by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` elements in occupied memory must be moved out or dropped.
    pub unsafe fn advance(&mut self, count: usize) {
        self.counter.advance_head(count);
    }

    /// Returns a pair of slices which contain, in order, the contents of the ring buffer.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let (left, right) = self.as_uninit_slices();
            (slice_assume_init_ref(left), slice_assume_init_ref(right))
        }
    }

    /// Returns a pair of slices which contain, in order, the contents of the ring buffer.
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            let (left, right) = self.as_mut_uninit_slices();
            (slice_assume_init_mut(left), slice_assume_init_mut(right))
        }
    }

    /// Removes latest element from the ring buffer and returns it.
    /// Returns `None` if the ring buffer is empty.
    pub fn pop(&mut self) -> Option<T> {
        if !self.is_empty() {
            let elem = unsafe { self.read() };
            unsafe { self.advance(1) };
            Some(elem)
        } else {
            None
        }
    }

    /// Returns iterator that removes elements one by one from the ring buffer.
    pub fn pop_iter(&mut self) -> LocalPopIterator<'a, '_, T, C> {
        LocalPopIterator { consumer: self }
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

    /// Removes exactly `count` elements from the head of ring buffer and drops them.
    ///
    /// *In debug mode panics if `count` is greater than number of elements stored in the buffer.*
    fn skip_unchecked(&mut self, count: usize) {
        let (left, right) = unsafe { self.as_mut_uninit_slices() };
        debug_assert!(count <= left.len() + right.len());
        for elem in left.iter_mut().chain(right.iter_mut()).take(count) {
            unsafe { ptr::drop_in_place(elem.as_mut_ptr()) };
        }
        unsafe { self.advance(count) };
    }

    /// Removes at most `n` and at least `min(n, Consumer::len())` items from the buffer and safely drops them.
    ///
    /// Returns the number of deleted items.
    pub fn skip(&mut self, count: usize) -> usize {
        let count = cmp::min(count, self.len());
        self.skip_unchecked(count);
        count
    }

    /// Removes all items from the buffer and safely drops them.
    ///
    /// Returns the number of deleted items.
    pub fn clear(&mut self) -> usize {
        let count = self.len();
        self.skip_unchecked(self.len());
        count
    }

    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns count of elements been moved.
    pub fn transfer_to<'b, Cd: Container<T>>(
        &mut self,
        other: &mut LocalProducer<'b, T, Cd>,
        count: Option<usize>,
    ) -> usize {
        transfer_local(self, other, count)
    }
}

pub struct LocalPopIterator<'a, 'b: 'a, T, C: Container<T>> {
    consumer: &'b mut LocalConsumer<'a, T, C>,
}

impl<'a, 'b: 'a, T, C: Container<T>> Iterator for LocalPopIterator<'a, 'b, T, C> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.consumer.pop()
    }
}

impl<'a, T: Copy, C: Container<T>> LocalConsumer<'a, T, C> {
    /// Removes first elements from the ring buffer and writes them into a slice.
    /// Elements should be `Copy`.
    ///
    /// On success returns count of elements been removed from the ring buffer.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> usize {
        let (left, right) = unsafe { self.as_uninit_slices() };
        let count = if elems.len() < left.len() {
            unsafe { write_uninit_slice(elems, &left[..elems.len()]) };
            elems.len()
        } else {
            let (left_elems, elems) = elems.split_at_mut(left.len());
            unsafe { write_uninit_slice(left_elems, left) };
            left.len()
                + if elems.len() < right.len() {
                    unsafe { write_uninit_slice(elems, &right[..elems.len()]) };
                    elems.len()
                } else {
                    unsafe { write_uninit_slice(&mut elems[..right.len()], right) };
                    right.len()
                }
        };
        unsafe { self.advance(count) };
        count
    }
}

#[cfg(feature = "std")]
impl<'a, C: Container<u8>> LocalConsumer<'a, u8, C> {
    pub fn write_into<S: Write>(
        &mut self,
        writer: &mut S,
        count: Option<usize>,
    ) -> io::Result<usize> {
        let (left, _) = unsafe { self.as_uninit_slices() };
        let count = cmp::min(count.unwrap_or(left.len()), left.len());
        let left_init = unsafe { slice_assume_init_ref(&left[..count]) };

        let write_count = writer.write(left_init)?;
        assert!(write_count <= count);
        unsafe { self.advance(write_count) };
        Ok(write_count)
    }
}

#[cfg(feature = "std")]
impl<'a, C: Container<u8>> Read for LocalConsumer<'a, u8, C> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.pop_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }
}
