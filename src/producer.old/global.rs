use super::LocalProducer;
use crate::{
    consumer::Consumer,
    counter::Counter,
    ring_buffer::{RingBuffer, RingBufferRef},
    transfer::transfer,
};
use core::marker::PhantomData;

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Producer part of ring buffer.
pub struct Producer<T, R: RingBufferRef<T>> {
    ring_buffer_ref: R,
    _phantom: PhantomData<T>,
}

impl<T, R: RingBufferRef<T>> Producer<T, R> {
    /// Creates producer from the ring buffer reference.
    ///
    /// # Safety
    ///
    /// There must be no another producer constructed from the same ring buffer.
    pub unsafe fn new(ring_buffer_ref: R) -> Self {
        Self {
            ring_buffer_ref,
            _phantom: PhantomData,
        }
    }

    /// Returns reference to the underlying ring buffer.
    #[inline]
    pub fn ring_buffer(&self) -> &R::RingBuffer {
        self.ring_buffer_ref.deref()
    }

    /// Consumes `self` and returns underlying ring buffer reference.
    pub fn into_ring_buffer_ref(self) -> R {
        self.ring_buffer_ref
    }

    /// Returns local producer that effectively freezes current ring buffer state.
    ///
    /// For more details see `LocalProducer`.
    pub fn acquire(&mut self) -> LocalProducer<'_, T, R::Counter> {
        unsafe {
            LocalProducer::new(
                self.ring_buffer().data(),
                self.ring_buffer().counter().acquire_tail(),
            )
        }
    }

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.ring_buffer().capacity()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// The result is relevant until you push items to the producer.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ring_buffer().counter().is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring activity of the consumer.*
    #[inline]
    pub fn is_full(&self) -> bool {
        self.ring_buffer().counter().is_full()
    }

    /// The number of items stored in the buffer.
    ///
    /// Actual number may be equal to or less than the returned value.
    pub fn len(&self) -> usize {
        self.ring_buffer().counter().occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// Actual number may be equal to or greater than the returning value.
    pub fn remaining(&self) -> usize {
        self.ring_buffer().counter().vacant_len()
    }

    /// Appends an item to the ring buffer.
    ///
    /// On failure returns an `Err` containing the item that hasn't been appended.
    pub fn push(&mut self, elem: T) -> Result<(), T> {
        self.acquire().push(elem)
    }

    /// Appends items from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    ///
    /// Returns count of items been appended to the ring buffer.
    ///
    /// *Inserted items are commited to the ring buffer all at once in the end,*
    /// *e.g. when buffer is full or iterator has ended.*
    pub fn push_iter<I: Iterator<Item = T>>(&mut self, iter: &mut I) -> usize {
        self.acquire().push_iter(iter)
    }

    /// Removes at most `count` items from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible items will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns number of items been moved.
    pub fn transfer_from<Rc: RingBufferRef<T>>(
        &mut self,
        consumer: &mut Consumer<T, Rc>,
        count: Option<usize>,
    ) -> usize {
        transfer(consumer, self, count)
    }
}

impl<T: Copy, R: RingBufferRef<T>> Producer<T, R> {
    /// Appends items from slice to the ring buffer.
    /// Elements should be `Copy`.
    ///
    /// Returns count of items been appended to the ring buffer.
    pub fn push_slice(&mut self, elems: &[T]) -> usize {
        self.acquire().push_slice(elems)
    }
}

#[cfg(feature = "std")]
impl<R: RingBufferRef<u8>> Producer<u8, R> {
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
        self.acquire().read_from(reader, count)
    }
}

#[cfg(feature = "std")]
impl<R: RingBufferRef<u8>> Write for Producer<u8, R> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.acquire().write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
