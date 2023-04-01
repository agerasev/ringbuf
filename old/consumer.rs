use crate::{
    ring_buffer::{RbBase, RbRead, RbReadCache, RbRef, RbWrap},
    utils::{slice_assume_init_mut, slice_assume_init_ref, write_uninit_slice},
};
use core::{cmp, iter::ExactSizeIterator, marker::PhantomData, mem::MaybeUninit};

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Consumer part of ring buffer.
///
/// # Mode
///
/// It can operate in immediate (by default) or postponed mode.
/// Mode could be switched using [`Self::postponed`]/[`Self::into_postponed`] and [`Self::into_immediate`] methods.
///
/// + In immediate mode removed and inserted items are automatically synchronized with the other end.
/// + In postponed mode synchronization occurs only when [`Self::sync`] or [`Self::into_immediate`] is called or when `Self` is dropped.
///   The reason to use postponed mode is that multiple subsequent operations are performed faster due to less frequent cache synchronization.
pub struct Consumer<T, R: RbRef>
where
    R::Rb: RbRead<T>,
{
    target: R,
    _phantom: PhantomData<T>,
}

impl<T, R: RbRef> Consumer<T, R>
where
    R::Rb: RbRead<T>,
{
    /// Creates consumer from the ring buffer reference.
    ///
    /// # Safety
    ///
    /// There must be only one consumer containing the same ring buffer reference.
    pub unsafe fn new(target: R) -> Self {
        Self {
            target,
            _phantom: PhantomData,
        }
    }

    /// Returns reference to the underlying ring buffer.
    #[inline]
    pub fn rb(&self) -> &R::Rb {
        &self.target
    }

    /// Consumes `self` and returns underlying ring buffer reference.
    pub fn into_rb_ref(self) -> R {
        self.target
    }

    /// Returns postponed consumer that borrows [`Self`].
    pub fn postponed(&mut self) -> Consumer<T, RbWrap<RbReadCache<T, &R::Rb>>> {
        unsafe { Consumer::new(RbWrap(RbReadCache::new(&self.target))) }
    }

    /// Transforms [`Self`] into postponed consumer.
    pub fn into_postponed(self) -> Consumer<T, RbWrap<RbReadCache<T, R>>> {
        unsafe { Consumer::new(RbWrap(RbReadCache::new(self.target))) }
    }

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.target.capacity_nonzero().get()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// *The result may become irrelevant at any time because of concurring producer activity.*
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.target.is_empty()
    }

    /// Checks if the ring buffer is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.target.is_full()
    }

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater than the returned value because of concurring producer activity.*
    #[inline]
    pub fn len(&self) -> usize {
        self.target.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be less than the returned value because of concurring producer activity.*
    #[inline]
    pub fn free_len(&self) -> usize {
        self.target.vacant_len()
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
    #[inline]
    pub unsafe fn as_uninit_slices(&self) -> (&[MaybeUninit<T>], &[MaybeUninit<T>]) {
        let (left, right) = self.target.occupied_slices();
        (left as &[_], right as &[_])
    }

    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// Same as [`Self::as_uninit_slices`].
    ///
    /// # Safety
    ///
    /// See [`Self::as_uninit_slices`].
    #[inline]
    pub unsafe fn as_mut_uninit_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.target.occupied_slices()
    }

    /// Moves `head` target by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied memory must be moved out or dropped.
    #[inline]
    pub unsafe fn advance(&mut self, count: usize) {
        self.target.advance_head(count);
    }

    /// Returns a pair of slices which contain, in order, the contents of the ring buffer.
    #[inline]
    pub fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let (left, right) = self.as_uninit_slices();
            (slice_assume_init_ref(left), slice_assume_init_ref(right))
        }
    }

    /// Returns a pair of mutable slices which contain, in order, the contents of the ring buffer.
    #[inline]
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
    ///
    /// Iterator provides only items that are available for consumer at the moment of `pop_iter` call, it will not contain new items added after it was created.
    ///
    /// *Information about removed items is commited to the buffer only when iterator is destroyed.*
    pub fn pop_iter(&mut self) -> PopIterator<'_, T, R> {
        PopIterator::new(&self.target)
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
# use ringbuf::HeapRb;
# fn main() {
let target = HeapRb::<i32>::new(8);
let (mut prod, mut cons) = target.split();

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
        assert_eq!(unsafe { self.target.skip_internal(Some(count)) }, count);
        count
    }

    /// Removes all items from the buffer and safely drops them.
    ///
    /// Returns the number of deleted items.
    pub fn clear(&mut self) -> usize {
        unsafe { self.target.skip_internal(None) }
    }
}

/// An iterator that removes items from the ring buffer.
pub struct PopIterator<'a, T, R: RbRef + ?Sized>
where
    R::Rb: RbRead<T>,
{
    target: &'a R,
    slices: (&'a [MaybeUninit<T>], &'a [MaybeUninit<T>]),
    initial_len: usize,
}

impl<'a, T, R: RbRef + ?Sized> PopIterator<'a, T, R>
where
    R::Rb: RbRead<T>,
{
    pub(crate) fn new(target: &'a R) -> Self {
        let slices = unsafe { target.occupied_slices() };
        Self {
            target,
            initial_len: slices.0.len() + slices.1.len(),
            slices: (slices.0, slices.1),
        }
    }
}

impl<'a, T, R: RbRef + ?Sized> Iterator for PopIterator<'a, T, R>
where
    R::Rb: RbRead<T>,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        match self.slices.0.len() {
            0 => None,
            n => {
                let item = unsafe { self.slices.0.get_unchecked(0).assume_init_read() };
                if n == 1 {
                    (self.slices.0, self.slices.1) = (self.slices.1, &[]);
                } else {
                    self.slices.0 = unsafe { self.slices.0.get_unchecked(1..n) };
                }
                Some(item)
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, T, R: RbRef + ?Sized> ExactSizeIterator for PopIterator<'a, T, R>
where
    R::Rb: RbRead<T>,
{
    fn len(&self) -> usize {
        self.slices.0.len() + self.slices.1.len()
    }
}

impl<'a, T, R: RbRef + ?Sized> Drop for PopIterator<'a, T, R>
where
    R::Rb: RbRead<T>,
{
    fn drop(&mut self) {
        unsafe { self.target.advance_head(self.initial_len - self.len()) };
    }
}

impl<T: Copy, R: RbRef> Consumer<T, R>
where
    R::Rb: RbRead<T>,
{
    /// Removes first items from the ring buffer and writes them into a slice.
    /// Elements must be [`Copy`].
    ///
    /// Returns count of items been removed from the ring buffer.
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

/// Postponed consumer.
pub type PostponedConsumer<T, R> = Consumer<T, RbWrap<RbReadCache<T, R>>>;

impl<T, R: RbRef> PostponedConsumer<T, R>
where
    R::Rb: RbRead<T>,
{
    /// Create new postponed consumer.
    ///
    /// # Safety
    ///
    /// There must be only one consumer containing the same ring buffer reference.
    pub unsafe fn new_postponed(target: R) -> Self {
        Consumer::new(RbWrap(RbReadCache::new(target)))
    }

    /// Synchronize changes with the ring buffer.
    ///
    /// Postponed consumer requires manual synchronization to make freed space visible for the producer.
    pub fn sync(&mut self) {
        self.target.0.sync();
    }

    /// Synchronize and transform back to immediate consumer.
    pub fn into_immediate(self) -> Consumer<T, R> {
        unsafe { Consumer::new(self.target.0.release()) }
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> Consumer<u8, R>
where
    R::Rb: RbRead<u8>,
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
impl<R: RbRef> Read for Consumer<u8, R>
where
    R::Rb: RbRead<u8>,
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
