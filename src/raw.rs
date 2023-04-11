use crate::traits::{Consumer, Observer, Producer, RingBuffer};
use core::{
    mem::MaybeUninit,
    num::NonZeroUsize,
    ops::{Deref, Range},
};

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
pub trait RawRb: Sized {
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

    /// Returns part of underlying raw ring buffer memory as slices.
    ///
    /// # Safety
    ///
    /// Only non-overlapping slices allowed to exist at the same time.
    unsafe fn slices(
        &self,
        start: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    );

    /// Read end position.
    fn read_index(&self) -> usize;

    /// Write end position.
    fn write_index(&self) -> usize;

    /// Sets the new **read** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::move_read_end` instead.
    unsafe fn set_read_index(&self, value: usize);

    /// Sets the new **write** position.
    ///
    /// # Safety
    ///
    /// This call must cohere with ring buffer data modification.
    ///
    /// It is recommended to use `Self::move_write_end` instead.
    unsafe fn set_write_index(&self, value: usize);
}

pub trait AsRaw {
    type Raw: RawRb;
    fn as_raw(&self) -> &Self::Raw;
}

impl<R: Deref> AsRaw for R
where
    R::Target: AsRaw,
{
    type Raw = <R::Target as AsRaw>::Raw;

    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self.deref().as_raw()
    }
}

pub unsafe trait ConsMarker: AsRaw {}
pub unsafe trait ProdMarker: AsRaw {}
pub unsafe trait RbMarker: ProdMarker + ConsMarker {}
unsafe impl<R: RbMarker> ProdMarker for R {}
unsafe impl<R: RbMarker> ConsMarker for R {}

impl<R: AsRaw> Observer for R {
    type Item = <R::Raw as RawRb>::Item;

    #[inline]
    fn capacity(&self) -> usize {
        self.as_raw().capacity().get()
    }

    fn occupied_len(&self) -> usize {
        let raw = self.as_raw();
        let modulus = raw.modulus();
        (modulus.get() + raw.write_index() - raw.read_index()) % modulus
    }

    fn vacant_len(&self) -> usize {
        let raw = self.as_raw();
        let modulus = raw.modulus();
        (raw.capacity().get() + raw.read_index() - raw.write_index()) % modulus
    }

    fn is_empty(&self) -> bool {
        let raw = self.as_raw();
        raw.read_index() == raw.write_index()
    }
}

impl<R: ProdMarker> Producer for R {
    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let raw = self.as_raw();
        let (first, second) =
            unsafe { raw.slices(raw.write_index(), raw.read_index() + raw.capacity().get()) };
        (first as &_, second as &_)
    }

    fn vacant_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        let raw = (self as &Self).as_raw();
        unsafe { raw.slices(raw.write_index(), raw.read_index() + raw.capacity().get()) }
    }

    unsafe fn advance_write(&mut self, count: usize) {
        let raw = self.as_raw();
        raw.set_write_index((raw.write_index() + count) % raw.modulus());
    }
}

impl<R: ConsMarker> Consumer for R {
    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let raw = self.as_raw();
        let (first, second) = unsafe { raw.slices(raw.read_index(), raw.write_index()) };
        (first as &_, second as &_)
    }

    unsafe fn occupied_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        let raw = (self as &Self).as_raw();
        raw.slices(raw.read_index(), raw.write_index())
    }

    unsafe fn advance_read(&mut self, count: usize) {
        let raw = self.as_raw();
        raw.set_read_index((raw.read_index() + count) % raw.modulus());
    }
}

impl<R: RbMarker> RingBuffer for R {}
