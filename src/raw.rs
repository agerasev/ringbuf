use crate::{Consumer, Observer, Producer, RingBuffer};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Range};

/// Returns a pair of ranges between `begin` and `end` positions in a ring buffer with specific `capacity`.
///
/// `begin` and `end` may be arbitrary large, but must satisfy the following condition: `0 <= (begin - end) % (2 * capacity) <= capacity`.
/// Actual positions are taken modulo `capacity`.
///
/// The first range starts from `begin`. If the first slice is empty then second slice is empty too.
pub fn ranges(capacity: NonZeroUsize, begin: usize, end: usize) -> (Range<usize>, Range<usize>) {
    let (head_quo, head_rem) = (begin / capacity, begin % capacity);
    let (tail_quo, tail_rem) = (end / capacity, end % capacity);

    if (head_quo + tail_quo) % 2 == 0 {
        (head_rem..tail_rem, 0..0)
    } else {
        (head_rem..capacity.get(), 0..tail_rem)
    }
}

/// Basic ring buffer functionality.
///
/// Provides an access to raw underlying memory and `read`/`write` counters.
///
/// *It is recommended not to use this trait directly. Use [`Producer`](`crate::Producer`) and [`Consumer`](`crate::Consumer`) instead.*
///
/// # Details
///
/// The ring buffer consists of an array (of `capacity` size) and two counters: `read` and `write`.
/// When an item is extracted from the ring buffer it is taken from the `read` position and after that `read` is incremented.
/// New item is appended to the `write` position and `write` is incremented after that.
///
/// The `read` and `write` counters are modulo `2 * capacity` (not just `capacity`).
/// It allows us to distinguish situations when the buffer is empty (`read == write`) and when the buffer is full (`write - read` modulo `2 * capacity` equals to `capacity`)
/// without using the space for an extra element in container.
/// And obviously we cannot store more than `capacity` items in the buffer, so `write - read` modulo `2 * capacity` is not allowed to be greater than `capacity`.
pub trait RawBase {
    type Item: Sized;

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity(&self) -> NonZeroUsize;

    /// Modulus for pointers to item in ring buffer storage.
    ///
    /// Equals to `2 * capacity`.
    #[inline]
    fn modulus(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(2 * self.capacity().get()) }
    }

    /// Get mutable slice of ring buffer data in specified `range`.
    ///
    /// # Safety
    ///
    /// `range` must be a subset of `0..capacity`.
    ///
    /// Slices with overlapping lifetimes must not overlap.
    #[allow(clippy::mut_from_ref)]
    unsafe fn slice(&self, range: Range<usize>) -> &mut [MaybeUninit<Self::Item>];

    /// Returns part of underlying raw ring buffer memory as slices.
    ///
    /// # Safety
    ///
    /// Only non-overlapping slices allowed to exist at the same time.
    #[inline]
    unsafe fn slices(
        &self,
        begin: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        let (first, second) = ranges(Self::capacity(self), begin, end);
        (self.slice(first), self.slice(second))
    }

    /// Read end position.
    fn read_end(&self) -> usize;

    /// Write ends position.
    fn write_end(&self) -> usize;
}

pub trait RawCons: RawBase {
    /// Sets the new **read** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::move_read_end` instead.
    unsafe fn set_read_end(&self, value: usize);
}

pub trait RawProd: RawBase {
    /// Sets the new **write** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::move_write_end` instead.
    unsafe fn set_write_end(&self, value: usize);
}

pub trait RawRb: RawProd + RawCons {}

impl<R: RawBase> Observer for R {
    type Item = R::Item;

    #[inline]
    fn capacity(&self) -> usize {
        R::capacity(self).get()
    }

    fn occupied_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.write_end() - self.read_end()) % modulus
    }

    fn vacant_len(&self) -> usize {
        let modulus = self.modulus();
        (self.capacity().get() + self.read_end() - self.write_end()) % modulus
    }

    fn is_empty(&self) -> bool {
        self.read_end() == self.write_end()
    }
}

impl<R: RawProd> Producer for R {
    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let (first, second) =
            unsafe { self.slices(self.write_end(), self.read_end() + self.capacity().get()) };
        (first as &_, second as &_)
    }

    fn vacant_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        unsafe { self.slices(self.write_end(), self.read_end() + self.capacity().get()) }
    }

    unsafe fn advance_write(&mut self, count: usize) {
        self.set_write_end((self.write_end() + count) % self.modulus());
    }
}

impl<R: RawCons> Consumer for R {
    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let (first, second) = unsafe { self.slices(self.read_end(), self.write_end()) };
        (first as &_, second as &_)
    }

    unsafe fn occupied_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.slices(self.read_end(), self.write_end())
    }

    unsafe fn advance_read(&mut self, count: usize) {
        self.set_read_end((self.read_end() + count) % self.modulus());
    }
}

impl<R: RawRb> RingBuffer for R {}
