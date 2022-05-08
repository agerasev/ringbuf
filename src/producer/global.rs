use super::LocalProducer;
use crate::{
    consumer::Consumer,
    ring_buffer::{AbstractRingBuffer, Counter, RingBufferRef},
    transfer::transfer,
};
use core::marker::PhantomData;

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Producer part of ring buffer.
///
/// Generic over item type, ring buffer container and ring buffer reference.
pub struct Producer<T, B, R>
where
    B: AbstractRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    ring_buffer: R,
    _phantom: PhantomData<(T, B)>,
}

impl<T, B, R> Producer<T, B, R>
where
    B: AbstractRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            ring_buffer,
            _phantom: PhantomData,
        }
    }

    pub fn acquire(&mut self) -> LocalProducer<'_, T> {
        unsafe {
            LocalProducer::new(
                self.ring_buffer.data(),
                self.ring_buffer.counter().acquire_tail(),
            )
        }
    }

    #[cfg(all(test, feature = "alloc"))]
    pub(crate) fn head(&self) -> usize {
        self.ring_buffer.counter().head()
    }
    #[cfg(all(test, feature = "alloc"))]
    pub(crate) fn tail(&self) -> usize {
        self.ring_buffer.counter().tail()
    }

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.ring_buffer.capacity().get()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// The result is relevant until you push items to the producer.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ring_buffer.counter().is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring activity of the consumer.*
    #[inline]
    pub fn is_full(&self) -> bool {
        self.ring_buffer.counter().is_full()
    }

    /// The number of elements stored in the buffer.
    ///
    /// Actual number may be equal to or less than the returned value.
    pub fn len(&self) -> usize {
        self.ring_buffer.counter().occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// Actual number may be equal to or greater than the returning value.
    pub fn remaining(&self) -> usize {
        self.ring_buffer.counter().vacant_len()
    }

    /// Appends an element to the ring buffer.
    ///
    /// On failure returns an `Err` containing the element that hasn't been appended.
    pub fn push(&mut self, elem: T) -> Result<(), T> {
        self.acquire().push(elem)
    }

    /// Appends elements from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    ///
    /// Returns count of elements been appended to the ring buffer.
    ///
    /// *Inserted elements are commited to the ring buffer all at once in the end,*
    /// *e.g. when buffer is full or iterator has ended.*
    pub fn push_iter<I: Iterator<Item = T>>(&mut self, iter: &mut I) -> usize {
        self.acquire().push_iter(iter)
    }

    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns number of elements been moved.
    pub fn transfer_from<Bs, Rs>(
        &mut self,
        other: &mut Consumer<T, Bs, Rs>,
        count: Option<usize>,
    ) -> usize
    where
        Bs: AbstractRingBuffer<T>,
        Rs: RingBufferRef<T, Bs>,
    {
        transfer(other, self, count)
    }
}

impl<T: Copy, B, R> Producer<T, B, R>
where
    B: AbstractRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    /// Appends elements from slice to the ring buffer.
    /// Elements should be `Copy`.
    ///
    /// Returns count of elements been appended to the ring buffer.
    pub fn push_slice(&mut self, elems: &[T]) -> usize {
        self.acquire().push_slice(elems)
    }
}

#[cfg(feature = "std")]
impl<B, R> Producer<u8, B, R>
where
    B: AbstractRingBuffer<u8>,
    R: RingBufferRef<u8, B>,
{
    /// Reads at most `count` bytes from `Read` instance and appends them to the ring buffer.
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
        self.acquire().read_from(reader, count)
    }
}

#[cfg(feature = "std")]
impl<B, R> Write for Producer<u8, B, R>
where
    B: AbstractRingBuffer<u8>,
    R: RingBufferRef<u8, B>,
{
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.acquire().write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
