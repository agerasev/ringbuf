use crate::{
    producer::GenericProducer,
    ring_buffer::{transfer, Container, RingBufferRef, StaticRingBuffer},
    utils::{slice_assume_init_mut, slice_assume_init_ref, write_uninit_slice},
};
use core::{cmp, marker::PhantomData, mem::MaybeUninit};

#[cfg(feature = "alloc")]
use crate::ring_buffer::RingBuffer;
#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Consumer part of ring buffer.
///
/// Generic over item type, ring buffer container and ring buffer reference.
pub struct GenericConsumer<T, C, R>
where
    C: Container<T>,
    R: RingBufferRef<T, C>,
{
    pub(crate) rb: R,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C, R> GenericConsumer<T, C, R>
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

impl<T, C, R> GenericConsumer<T, C, R>
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
    /// Actual number may be equal to or greater than the returned value.
    pub fn len(&self) -> usize {
        self.rb.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// Actual number may be equal to or less than the returning value.
    pub fn remaining(&self) -> usize {
        self.rb.vacant_len()
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
        let (left, right) = self.rb.occupied_slices();
        (left, right)
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
        self.rb.occupied_slices()
    }

    /// Moves `head` counter by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` elements in occupied memory must be moved out or dropped.
    pub unsafe fn advance(&mut self, count: usize) {
        self.rb.advance_head(count);
    }

    /// Returns a pair of slices which contain, in order, the contents of the ring buffer.
    ///
    /// *The slices may not include elements pushed to the buffer by concurring producer after the method call.*
    pub fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let (left, right) = self.as_uninit_slices();
            (slice_assume_init_ref(left), slice_assume_init_ref(right))
        }
    }

    /// Returns a pair of slices which contain, in order, the contents of the ring buffer.
    ///
    /// *The slices may not include elements pushed to the buffer by concurring producer after the method call.*
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            let (left, right) = self.as_mut_uninit_slices();
            (slice_assume_init_mut(left), slice_assume_init_mut(right))
        }
    }

    /// Removes latest element from the ring buffer and returns it.
    /// Returns `None` if the ring buffer is empty.
    pub fn pop(&mut self) -> Option<T> {
        let (left, _) = unsafe { self.as_uninit_slices() };
        match left.iter().next() {
            Some(place) => {
                let elem = unsafe { place.as_ptr().read() };
                unsafe { self.advance(1) };
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
    #[cfg_attr(
        feature = "alloc",
        doc = r##"
```rust
# extern crate ringbuf;
# use ringbuf::RingBuffer;
# fn main() {
let rb = RingBuffer::<i32>::new(8);
let (mut prod, mut cons) = rb.split();

assert_eq!(prod.push_iter(&mut (0..8)), 8);

assert_eq!(cons.skip(4), 4);
assert_eq!(cons.skip(8), 4);
assert_eq!(cons.skip(8), 0);
# }
```
"##
    )]
    pub fn skip(&mut self, count: usize) -> usize {
        let actual_count = cmp::min(count, self.len());
        self.rb.skip(actual_count);
        actual_count
    }

    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns count of elements been moved.
    pub fn transfer_to<Cd, Rd>(
        &mut self,
        other: &mut GenericProducer<T, Cd, Rd>,
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
    consumer: &'a mut GenericConsumer<T, C, R>,
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

impl<T: Copy, C, R> GenericConsumer<T, C, R>
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
impl<C, R> GenericConsumer<u8, C, R>
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
    /// If `write` is failed then original error is returned.
    // TODO: Add note about writing only one contiguous slice at once.
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
impl<C, R> Read for GenericConsumer<u8, C, R>
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

pub type StaticConsumer<'a, T, const N: usize> =
    GenericConsumer<T, [MaybeUninit<T>; N], &'a StaticRingBuffer<T, N>>;
#[cfg(feature = "alloc")]
pub type Consumer<T> = GenericConsumer<T, Vec<MaybeUninit<T>>, Arc<RingBuffer<T>>>;
