use super::LocalConsumer;
use crate::{
    counter::Counter,
    producer::GlobalProducer,
    ring_buffer::{Container, RingBufferRef},
    transfer::transfer,
};
use core::marker::PhantomData;

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Consumer part of ring buffer.
///
/// Generic over item type, ring buffer container and ring buffer reference.
pub struct GlobalConsumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    ring_buffer: R,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C, R> GlobalConsumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    pub(crate) fn new(ring_buffer: R) -> Self {
        Self {
            ring_buffer,
            _phantom: PhantomData,
        }
    }

    pub fn acquire(&mut self) -> LocalConsumer<'_, T, C> {
        LocalConsumer::new(&self.ring_buffer.storage, unsafe {
            self.ring_buffer.counter.acquire_head()
        })
    }

    #[cfg(all(test, feature = "alloc"))]
    pub(crate) fn head(&self) -> usize {
        self.ring_buffer.counter.head()
    }
    #[cfg(all(test, feature = "alloc"))]
    pub(crate) fn tail(&self) -> usize {
        self.ring_buffer.counter.tail()
    }

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    pub fn capacity(&self) -> usize {
        self.ring_buffer.capacity()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// The result is relevant until you push items to the producer.
    pub fn is_empty(&self) -> bool {
        self.ring_buffer.counter.is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring activity of the consumer.*
    pub fn is_full(&self) -> bool {
        self.ring_buffer.counter.is_full()
    }

    /// The number of elements stored in the buffer.
    ///
    /// Actual number may be equal to or greater than the returned value.
    pub fn len(&self) -> usize {
        self.ring_buffer.counter.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// Actual number may be equal to or less than the returning value.
    pub fn remaining(&self) -> usize {
        self.ring_buffer.counter.vacant_len()
    }

    /// Removes latest element from the ring buffer and returns it.
    /// Returns `None` if the ring buffer is empty.
    pub fn pop(&mut self) -> Option<T> {
        self.acquire().pop()
    }

    /// Returns iterator that removes elements one by one from the ring buffer.
    pub fn pop_iter(&mut self) -> PopIterator<T, C, R> {
        PopIterator { consumer: self }
    }

    /// Removes at most `n` and at least `min(n, Consumer::len())` items from the buffer and safely drops them.
    ///
    /// If there is no concurring producer activity then exactly `min(n, Consumer::len())` items are removed.
    ///
    /// Returns the number of deleted items.
    ///
    #[cfg_attr(
        feature = "alloc",
        doc = r##"
```rust
# extern crate ringbuf;
# use ringbuf::RingBuffer;
# fn main() {
let ring_buffer = RingBuffer::<i32>::new(8);
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
        self.acquire().skip(count)
    }

    /// Removes all items from the buffer and safely drops them.
    ///
    /// If there is concurring producer activity then the buffer may be not empty after this call.
    ///
    /// Returns the number of deleted items.
    pub fn clear(&mut self) -> usize {
        self.acquire().clear()
    }

    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns count of elements been moved.
    pub fn transfer_to<Cd, Rd>(
        &mut self,
        other: &mut GlobalProducer<T, Cd, Rd>,
        count: Option<usize>,
    ) -> usize
    where
        Cd: Container<T>,
        Rd: RingBufferRef<T, Cd>,
    {
        transfer(self, other, count)
    }
}

pub struct PopIterator<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    consumer: &'a mut GlobalConsumer<T, C, R>,
}

impl<'a, T, C, R> Iterator for PopIterator<'a, T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.consumer.pop()
    }
}

impl<T: Copy, C, R> GlobalConsumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    /// Removes first elements from the ring buffer and writes them into a slice.
    /// Elements should be `Copy`.
    ///
    /// On success returns count of elements been removed from the ring buffer.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> usize {
        self.acquire().pop_slice(elems)
    }
}

#[cfg(feature = "std")]
impl<C, R> GlobalConsumer<u8, C, R>
where
    C: Container<u8>,
    R: RingBufferRef<u8, C>,
{
    /// Removes at most first `count` bytes from the ring buffer and writes them into a `Write` instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    ///
    /// Returns `Ok(n)` if `write` succeeded. `n` is number of bytes been written.
    /// `n == 0` means that either `write` returned zero or ring buffer is empty.
    ///
    /// If `write` is failed then original error is returned.
    // TODO: Add note about writing only one contiguous slice at once.
    pub fn write_into<S: Write>(
        &mut self,
        writer: &mut S,
        count: Option<usize>,
    ) -> io::Result<usize> {
        self.acquire().write_into(writer, count)
    }
}

#[cfg(feature = "std")]
impl<C, R> Read for GlobalConsumer<u8, C, R>
where
    C: Container<u8>,
    R: RingBufferRef<u8, C>,
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.acquire().read(buffer)
    }
}
