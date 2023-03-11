use crate::{
    ring_buffer::{RbBase, RbRef, RbWrap, RbWrite, RbWriteCache},
    utils::write_slice,
};
use core::{marker::PhantomData, mem::MaybeUninit};

#[cfg(feature = "std")]
use crate::utils::slice_assume_init_mut;
#[cfg(feature = "std")]
use core::cmp;
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

/// Producer part of ring buffer.
///
/// # Mode
///
/// It can operate in immediate (by default) or postponed mode.
/// Mode could be switched using [`Self::postponed`]/[`Self::into_postponed`] and [`Self::into_immediate`] methods.
///
/// + In immediate mode removed and inserted items are automatically synchronized with the other end.
/// + In postponed mode synchronization occurs only when [`Self::sync`] or [`Self::into_immediate`] is called or when `Self` is dropped.
///   The reason to use postponed mode is that multiple subsequent operations are performed faster due to less frequent cache synchronization.
pub struct Producer<T, R: RbRef>
where
    R::Rb: RbWrite<T>,
{
    target: R,
    _phantom: PhantomData<T>,
}

impl<T, R: RbRef> Producer<T, R>
where
    R::Rb: RbWrite<T>,
{
    /// Creates producer from the ring buffer reference.
    ///
    /// # Safety
    ///
    /// There must be only one producer containing the same ring buffer reference.
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

    /// Returns postponed producer that borrows [`Self`].
    pub fn postponed(&mut self) -> PostponedProducer<T, &R::Rb> {
        unsafe { Producer::new(RbWrap(RbWriteCache::new(&self.target))) }
    }

    /// Transforms [`Self`] into postponed producer.
    pub fn into_postponed(self) -> PostponedProducer<T, R> {
        unsafe { Producer::new(RbWrap(RbWriteCache::new(self.target))) }
    }

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.target.capacity_nonzero().get()
    }

    /// Checks if the ring buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.target.is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring consumer activity.*
    #[inline]
    pub fn is_full(&self) -> bool {
        self.target.is_full()
    }

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be less than the returned value because of concurring consumer activity.*
    #[inline]
    pub fn len(&self) -> usize {
        self.target.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be greater than the returning value because of concurring consumer activity.*
    #[inline]
    pub fn free_len(&self) -> usize {
        self.target.vacant_len()
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
    #[inline]
    pub unsafe fn free_space_as_slices(
        &mut self,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.target.vacant_slices()
    }

    /// Moves `tail` counter by `count` places.
    ///
    /// # Safety
    ///
    /// First `count` items in free space must be initialized.
    #[inline]
    pub unsafe fn advance(&mut self, count: usize) {
        self.target.advance_tail(count)
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
    /// *Inserted items are committed to the ring buffer all at once in the end,*
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
}

impl<T: Copy, R: RbRef> Producer<T, R>
where
    R::Rb: RbWrite<T>,
{
    /// Appends items from slice to the ring buffer.
    /// Elements must be [`Copy`].
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

/// Postponed producer.
pub type PostponedProducer<T, R> = Producer<T, RbWrap<RbWriteCache<T, R>>>;

impl<T, R: RbRef> PostponedProducer<T, R>
where
    R::Rb: RbWrite<T>,
{
    /// Create new postponed producer.
    ///
    /// # Safety
    ///
    /// There must be only one producer containing the same ring buffer reference.
    pub unsafe fn new_postponed(target: R) -> Self {
        Producer::new(RbWrap(RbWriteCache::new(target)))
    }

    /// Synchronize changes with the ring buffer.
    ///
    /// Postponed producer requires manual synchronization to make pushed items visible for the consumer.
    pub fn sync(&mut self) {
        self.target.0.sync();
    }

    /// Don't publish and drop items inserted since last synchronization.
    pub fn discard(&mut self) {
        self.target.0.discard();
    }

    /// Synchronize and transform back to immediate producer.
    pub fn into_immediate(self) -> Producer<T, R> {
        unsafe { Producer::new(self.target.0.release()) }
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> Producer<u8, R>
where
    R::Rb: RbWrite<u8>,
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
impl<R: RbRef> Write for Producer<u8, R>
where
    R::Rb: RbWrite<u8>,
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

impl<R: RbRef> core::fmt::Write for Producer<u8, R>
where
    R::Rb: RbWrite<u8>,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let n = self.push_slice(s.as_bytes());
        if n != s.len() {
            Err(core::fmt::Error::default())
        } else {
            Ok(())
        }
    }
}
