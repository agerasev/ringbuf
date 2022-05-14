use crate::{
    counter::{Counter, LocalHeadCounter},
    producer::LocalProducer,
    transfer::transfer_local,
    utils::{slice_assume_init_mut, slice_assume_init_ref, write_uninit_slice},
};
use core::{cmp, mem::MaybeUninit, ptr, slice};

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// HeapConsumer part of ring buffer.
///
/// Generic over item type, ring buffer container and ring buffer reference.
pub struct LocalConsumer<'a, T, S: Counter> {
    data: &'a mut [MaybeUninit<T>],
    counter: LocalHeadCounter<'a, S>,
}

impl<'a, T, S: Counter> LocalConsumer<'a, T, S> {
    pub(crate) fn new(data: &'a mut [MaybeUninit<T>], counter: LocalHeadCounter<'a, S>) -> Self {
        Self { data, counter }
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.counter.is_empty()
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        self.counter.is_full()
    }

    /// The number of items stored in the buffer.
    pub fn len(&self) -> usize {
        self.counter.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    pub fn remaining(&self) -> usize {
        self.counter.vacant_len()
    }

    /// Provides a direct access to the ring buffer occupied memory.
    /// The difference from [`Self::as_slices`] is that this method provides slices of [`MaybeUninit<T>`], so items may be moved out of slices.  
    ///
    /// Returns a pair of slices of stored items, the second one may be empty.
    /// Elements with lower indices in slice are older. First slice contains older items that second one.
    ///
    /// # Safety
    ///
    /// All items are initialized. Elements must be removed starting from the beginning of first slice.
    /// When all items are removed from the first slice then items must be removed from the beginning of the second slice.
    ///
    /// *This method must be followed by [`Self::advance`] call with the number of items being removed previously as argument.*
    /// *No other mutating calls allowed before that.*
    pub unsafe fn as_uninit_slices(&self) -> (&[MaybeUninit<T>], &[MaybeUninit<T>]) {
        let ranges = self.counter.occupied_ranges();
        let ptr = self.data.as_ptr();
        (
            slice::from_raw_parts(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }
    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// See [`Self::as_uninit_slices`] for details.
    ///
    /// # Safety
    ///
    /// The same as for [`Self::as_uninit_slices`] except that items could be modified without being removed.
    pub unsafe fn as_mut_uninit_slices(
        &mut self,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let ranges = self.counter.occupied_ranges();
        let ptr = self.data.as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// Moves `head` counter by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied memory must be moved out or dropped.
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

    /// Returns a pair of mutable slices which contain, in order, the contents of the ring buffer.
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            let (left, right) = self.as_mut_uninit_slices();
            (slice_assume_init_mut(left), slice_assume_init_mut(right))
        }
    }

    /// Removes latest item from the ring buffer and returns it.
    /// Returns `None` if the ring buffer is empty.
    pub fn pop(&mut self) -> Option<T> {
        if !self.is_empty() {
            let elem = unsafe {
                self.as_uninit_slices()
                    .0
                    .get_unchecked(0)
                    .assume_init_read()
            };
            unsafe { self.advance(1) };
            Some(elem)
        } else {
            None
        }
    }

    /// Local version of [`super::Consumer::pop_iter`].
    pub fn pop_iter(&mut self) -> LocalPopIterator<'a, '_, T, S> {
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

    /// Removes exactly `count` items from the head of ring buffer and drops them.
    ///
    /// *In debug mode panics if `count` is greater than number of items stored in the buffer.*
    fn skip_unchecked(&mut self, count: usize) {
        let (left, right) = unsafe { self.as_mut_uninit_slices() };
        debug_assert!(count <= left.len() + right.len());
        for elem in left.iter_mut().chain(right.iter_mut()).take(count) {
            unsafe { ptr::drop_in_place(elem.as_mut_ptr()) };
        }
        unsafe { self.advance(count) };
    }

    /// Removes at most `n` and at least `min(n, HeapConsumer::len())` items from the buffer and safely drops them.
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

    /// Local version of [`super::Consumer::transfer_to`].
    pub fn transfer_to<'b, Sp: Counter>(
        &mut self,
        producer: &mut LocalProducer<'b, T, Sp>,
        count: Option<usize>,
    ) -> usize {
        transfer_local(self, producer, count)
    }
}

/// Local version of [`PopIterator`](`super::PopIterator`).
pub struct LocalPopIterator<'a, 'b: 'a, T, S: Counter> {
    consumer: &'b mut LocalConsumer<'a, T, S>,
}

impl<'a, 'b: 'a, T, S: Counter> Iterator for LocalPopIterator<'a, 'b, T, S> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.consumer.pop()
    }
}

impl<'a, T: Copy, S: Counter> LocalConsumer<'a, T, S> {
    /// Local version of [`super::Consumer::pop_slice`].
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
impl<'a, S: Counter> LocalConsumer<'a, u8, S> {
    /// Local version of [`super::Consumer::write_into`].
    pub fn write_into<P: Write>(
        &mut self,
        writer: &mut P,
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
impl<'a, S: Counter> Read for LocalConsumer<'a, u8, S> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.pop_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }
}
