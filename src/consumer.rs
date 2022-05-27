use crate::{
    ring_buffer::{RingBufferBase, RingBufferRead},
    utils::{slice_assume_init_mut, slice_assume_init_ref, write_uninit_slice},
};
use core::{cmp, marker::PhantomData, mem::MaybeUninit, ops::Deref, slice};

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Consumer part of ring buffer.
///
/// The difference from [`Consumer`](`crate::Consumer`) is that all changes is postponed
/// until [`Self::sync`] or [`Self::release`] is called or `Self` is dropped.
pub struct Consumer<T, R> {
    ring_buffer: R,
    _phantom: PhantomData<T>,
}

impl<T, R: Deref> Consumer<T, R>
where
    R::Target: RingBufferRead<T>,
{
    /// Creates consumer from the ring buffer reference.
    ///
    /// # Safety
    ///
    /// There must be no another consumer containing the same ring buffer reference.
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            ring_buffer,
            _phantom: PhantomData,
        }
    }

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    pub fn capacity(&self) -> usize {
        self.ring_buffer.capacity().get()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// *The result may become irrelevant at any time because of concurring producer activity.*
    pub fn is_empty(&self) -> bool {
        self.ring_buffer.is_empty()
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        self.ring_buffer.is_full()
    }

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater than the returned value because of concurring producer activity.*
    pub fn len(&self) -> usize {
        self.ring_buffer.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be less than the returned value because of concurring producer activity.*
    pub fn free_len(&self) -> usize {
        self.ring_buffer.vacant_len()
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
        let ranges = self.ring_buffer.occupied_ranges();
        let ptr = self.ring_buffer.data().as_ptr();
        (
            slice::from_raw_parts(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// Same as [`Self::as_uninit_slices`].
    ///
    /// # Safety
    ///
    /// See [`Self::as_uninit_slices`].
    pub unsafe fn as_mut_uninit_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let ranges = self.ring_buffer.occupied_ranges();
        let ptr = self.ring_buffer.data().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// Moves `head` ring_buffer by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied memory must be moved out or dropped.
    pub unsafe fn advance(&mut self, count: usize) {
        self.ring_buffer.advance_head(count);
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
    ///
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

    /// Returns an iterator that removes items one by one from the ring buffer.
    pub fn pop_iter(&mut self) -> PopIterator<'_, T, R> {
        PopIterator { consumer: self }
    }

    /// Returns a front-to-back iterator containing references to items in the ring buffer.
    ///
    /// This iterator does not remove items out of the ring buffer.
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        let (left, right) = self.as_slices();
        left.iter().chain(right.iter())
    }

    /// Returns a front-to-back iterator that returns mutable references to items in the ring buffer.
    ///
    /// This iterator does not remove items out of the ring buffer.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        let (left, right) = self.as_mut_slices();
        left.iter_mut().chain(right.iter_mut())
    }

    /// Removes at most `n` and at least `min(n, Self::len())` items from the buffer and safely drops them.
    ///
    /// If there is no concurring producer activity then exactly `min(n, Self::len())` items are removed.
    ///
    /// Returns the number of deleted items.
    ///
    #[cfg_attr(
        feature = "alloc",
        doc = r##"
```rust
# extern crate ringbuf;
# use ringbuf::HeapRingBuffer;
# fn main() {
let ring_buffer = HeapRingBuffer::<i32>::new(8);
let (mut prod, mut cons) = ring_buffer.split();

assert_eq!(prod.push_iter(&mut (0..8)), 8);

assert_eq!(cons.skip(4), 4);
assert_eq!(cons.skip(8), 4);
assert_eq!(cons.skip(8), 0);
# }
```
"##
    )]
    pub fn skip(&mut self, count: usize) -> usize {
        let count = cmp::min(count, self.len());
        assert_eq!(unsafe { self.ring_buffer.skip(Some(count)) }, count);
        count
    }

    /// Removes all items from the buffer and safely drops them.
    ///
    /// Returns the number of deleted items.
    pub fn clear(&mut self) -> usize {
        unsafe { self.ring_buffer.skip(None) }
    }
    /*
    /// See [`Consumer::transfer_to`](`super::Consumer::transfer_to`).
    pub fn transfer_to<'b, Sp: Counter>(
        &mut self,
        producer: &mut LocalProducer<'b, T, Sp>,
        count: Option<usize>,
    ) -> usize {
        transfer_local(self, producer, count)
    }
    */
}

/// See [`PopIterator`](`super::PopIterator`).
pub struct PopIterator<'a, T, R: Deref>
where
    R::Target: RingBufferRead<T>,
{
    consumer: &'a mut Consumer<T, R>,
}

impl<'a, T, R: Deref> Iterator for PopIterator<'a, T, R>
where
    R::Target: RingBufferRead<T>,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.consumer.pop()
    }
}

impl<'a, T: Copy, R: Deref> Consumer<T, R>
where
    R::Target: RingBufferRead<T>,
{
    /// Removes first items from the ring buffer and writes them into a slice.
    /// Elements must be [`Copy`].
    ///
    /// On success returns count of items been removed from the ring buffer.
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
impl<'a, R: Deref> Consumer<u8, R>
where
    R::Target: RingBufferRead<u8>,
{
    /// Removes at most first `count` bytes from the ring buffer and writes them into a [`Write`] instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    ///
    /// Returns `Ok(n)` if `write` succeeded. `n` is number of bytes been written.
    /// `n == 0` means that either `write` returned zero or ring buffer is empty.
    ///
    /// If `write` is failed then original error is returned. In this case it is guaranteed that no items was written to the writer.
    /// To achieve this we write only one contiguous slice at once. So this call may write less than `len` items even if the writer is ready to get more.
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
impl<'a, R: Deref> Read for Consumer<u8, R>
where
    R::Target: RingBufferRead<u8>,
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
