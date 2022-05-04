use crate::{
    consumer::GenericConsumer,
    ring_buffer::{transfer, Container, RingBufferRef, StaticRingBuffer},
    utils::write_slice,
};
use core::{marker::PhantomData, mem::MaybeUninit};

#[cfg(feature = "alloc")]
use crate::ring_buffer::RingBuffer;
#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

#[cfg(feature = "std")]
use crate::utils::slice_assume_init_mut;
#[cfg(feature = "std")]
use core::cmp;
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Producer part of ring buffer.
///
/// Generic over item type, ring buffer container and ring buffer reference.
pub struct GenericProducer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    pub(crate) rb: R,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C, R> GenericProducer<T, C, R>
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

impl<T, C, R> GenericProducer<T, C, R>
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

    /// The number of elements stored in the buffer.
    ///
    /// Actual number may be equal to or less than the returned value.
    pub fn len(&self) -> usize {
        self.rb.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// Actual number may be equal to or greater than the returning value.
    pub fn remaining(&self) -> usize {
        self.rb.vacant_len()
    }

    /// Provides a direct access to the ring buffer vacant memory.
    /// Returns a pair of slices of uninitialized memory, the second one may be empty.
    ///
    /// # Safety
    ///
    /// Vacant memory is uninitialized. Initialized elements must be put starting from the beginning of first slice.
    /// When first slice is fully filled then elements must be put to the beginning of the second slice.
    ///
    /// *This method must be followed by `Self::advance` call with the number of elements being put previously as argument.*
    /// *No other mutating calls allowed before that.*
    pub unsafe fn free_space_as_slices(
        &mut self,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.rb.vacant_slices()
    }

    /// Moves `tail` counter by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` elements in free space must be initialized.
    pub unsafe fn advance(&mut self, count: usize) {
        self.rb.advance_tail(count);
    }

    /// Appends an element to the ring buffer.
    ///
    /// On failure returns an `Err` containing the element that hasn't been appended.
    pub fn push(&mut self, elem: T) -> Result<(), T> {
        if !self.is_full() {
            unsafe { self.rb.write_tail(elem) };
            unsafe { self.advance(1) };
            Ok(())
        } else {
            Err(elem)
        }
    }

    /// Appends elements from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    ///
    /// Returns count of elements been appended to the ring buffer.
    ///
    /// *Inserted elements are commited to the ring buffer all at once in the end,*
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

    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns number of elements been moved.
    pub fn transfer_from<Cs, Rs>(
        &mut self,
        other: &mut GenericConsumer<T, Cs, Rs>,
        count: Option<usize>,
    ) -> usize
    where
        Cs: Container<T>,
        Rs: RingBufferRef<T, Cs>,
    {
        transfer(other, self, count)
    }
}

impl<T: Copy, C, R> GenericProducer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    /// Appends elements from slice to the ring buffer.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// Returns count of elements been appended to the ring buffer.
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
impl<C, R> GenericProducer<u8, C, R>
where
    C: Container<u8>,
    R: RingBufferRef<u8, C>,
{
    /// Reads at most `count` bytes
    /// from [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) instance
    /// and appends them to the ring buffer.
    /// If `count` is `None` then as much as possible bytes will be read.
    ///
    /// Returns `Ok(n)` if `read` succeeded. `n` is number of bytes been read.
    /// `n == 0` means that either `read` returned zero or ring buffer is full.
    ///
    /// If `read` is failed or returned an invalid number then error is returned.
    // TODO: Add note about reading only one contiguous slice at once.
    pub fn read_from<S: Read>(
        &mut self,
        reader: &mut S,
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
impl<C, R> Write for GenericProducer<u8, C, R>
where
    C: Container<u8>,
    R: RingBufferRef<u8, C>,
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

/// Producer that holds reference to `StaticRingBuffer`.
pub type StaticProducer<'a, T, const N: usize> =
    GenericProducer<T, [MaybeUninit<T>; N], &'a StaticRingBuffer<T, N>>;

/// Producer that holds `Arc<RingBuffer>`.
#[cfg(feature = "alloc")]
pub type Producer<T> = GenericProducer<T, Vec<MaybeUninit<T>>, Arc<RingBuffer<T>>>;
