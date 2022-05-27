use crate::{
    ring_buffer::{RingBufferBase, RingBufferWrite},
    utils::write_slice,
};
use core::{marker::PhantomData, mem::MaybeUninit, ops::Deref};

#[cfg(feature = "std")]
use crate::utils::slice_assume_init_mut;
#[cfg(feature = "std")]
use core::cmp;
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Producer part of ring buffer.
///
/// Generic over item type, ring buffer container and ring buffer reference.
pub struct Producer<T, R: Deref>
where
    R::Target: RingBufferWrite<T>,
{
    ring_buffer: R,
    _phantom: PhantomData<T>,
}

impl<T, R: Deref> Producer<T, R>
where
    R::Target: RingBufferWrite<T>,
{
    /// Creates producer from the ring buffer reference.
    ///
    /// # Safety
    ///
    /// There must be no another producer constructed from the same ring buffer.
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            ring_buffer,
            _phantom: PhantomData,
        }
    }

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.ring_buffer.capacity().get()
    }

    /// Checks if the ring buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ring_buffer.is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring consumer activity.*
    #[inline]
    pub fn is_full(&self) -> bool {
        self.ring_buffer.is_full()
    }

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be less than the returned value because of concurring consumer activity.*
    pub fn len(&self) -> usize {
        self.ring_buffer.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be greater than the returning value because of concurring consumer activity.*
    pub fn free_len(&self) -> usize {
        self.ring_buffer.vacant_len()
    }

    /// Provides a direct access to the ring buffer vacant memory.
    /// Returns a pair of slices of uninitialized memory, the second one may be empty.
    ///
    /// # Safety
    ///
    /// Vacant memory is uninitialized. Initialized items must be put starting from the beginning of first slice.
    /// When first slice is fully filled then items must be put to the beginning of the second slice.
    ///
    /// *This method must be followed by `Self::advance` call with the number of items being put previously as argument.*
    /// *No other mutating calls allowed before that.*
    pub unsafe fn free_space_as_slices(
        &mut self,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.ring_buffer.vacant_slices()
    }

    /// Moves `tail` counter by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` items in free space must be initialized.
    pub unsafe fn advance(&mut self, count: usize) {
        self.ring_buffer.advance_tail(count)
    }

    /// Appends an item to the ring buffer.
    ///
    /// On failure returns an `Err` containing the item that hasn't been appended.
    pub fn push(&mut self, elem: T) -> Result<(), T> {
        if !self.is_full() {
            unsafe {
                self.free_space_as_slices()
                    .0
                    .get_unchecked_mut(0)
                    .write(elem)
            };
            unsafe { self.advance(1) };
            Ok(())
        } else {
            Err(elem)
        }
    }

    /// Appends items from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    ///
    /// Returns count of items been appended to the ring buffer.
    ///
    /// *Inserted items are commited to the ring buffer all at once in the end,*
    /// *e.g. when buffer is full or iterator has ended.*
    pub fn push_iter<I: Iterator<Item = T>>(&mut self, iter: &mut I) -> usize {
        let (left, right) = unsafe { self.free_space_as_slices() };
        let mut count = 0;
        for place in left.iter_mut().chain(right.iter_mut()) {
            match iter.next() {
                Some(elem) => unsafe { place.as_mut_ptr().write(elem) },
                None => break,
            }
            count += 1;
        }
        unsafe { self.advance(count) };
        count
    }
    /*
    /// Removes at most `count` items from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible items will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns number of items been moved.
    pub fn transfer_from<'b, Sc: Counter>(
        &mut self,
        consumer: &mut LocalConsumer<'b, T, Sc>,
        count: Option<usize>,
    ) -> usize {
        transfer_local(consumer, self, count)
    }
    */
}

impl<'a, T: Copy, R: Deref> Producer<T, R>
where
    R::Target: RingBufferWrite<T>,
{
    /// Appends items from slice to the ring buffer.
    /// Elements should be `Copy`.
    ///
    /// Returns count of items been appended to the ring buffer.
    pub fn push_slice(&mut self, elems: &[T]) -> usize {
        let (left, right) = unsafe { self.free_space_as_slices() };
        let count = if elems.len() < left.len() {
            write_slice(&mut left[..elems.len()], elems);
            elems.len()
        } else {
            let (left_elems, elems) = elems.split_at(left.len());
            write_slice(left, left_elems);
            left.len()
                + if elems.len() < right.len() {
                    write_slice(&mut right[..elems.len()], elems);
                    elems.len()
                } else {
                    write_slice(right, &elems[..right.len()]);
                    right.len()
                }
        };
        unsafe { self.advance(count) };
        count
    }
}

#[cfg(feature = "std")]
impl<R: Deref> Producer<u8, R>
where
    R::Target: RingBufferWrite<u8>,
{
    /// Reads at most `count` bytes from `Read` instance and appends them to the ring buffer.
    /// If `count` is `None` then as much as possible bytes will be read.
    ///
    /// Returns `Ok(n)` if `read` succeeded. `n` is number of bytes been read.
    /// `n == 0` means that either `read` returned zero or ring buffer is full.
    ///
    /// If `read` is failed then original error is returned. In this case it is guaranteed that no items was read from the reader.
    /// To achieve this we read only one contiguous slice at once. So this call may read less than `remaining` items in the buffer even if the reader is ready to provide more.
    pub fn read_from<P: Read>(
        &mut self,
        reader: &mut P,
        count: Option<usize>,
    ) -> io::Result<usize> {
        let (left, _) = unsafe { self.free_space_as_slices() };
        let count = cmp::min(count.unwrap_or(left.len()), left.len());
        let left_init = unsafe { slice_assume_init_mut(&mut left[..count]) };

        let read_count = reader.read(left_init)?;
        assert!(read_count <= count);
        unsafe { self.advance(read_count) };
        Ok(read_count)
    }
}

#[cfg(feature = "std")]
impl<R: Deref> Write for Producer<u8, R>
where
    R::Target: RingBufferWrite<u8>,
{
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let n = self.push_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
