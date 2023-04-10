use crate::{
    raw::{RawBase, RawCons},
    utils::{slice_assume_init_mut, slice_assume_init_ref, write_uninit_slice},
    Observer,
};
use core::{iter::Chain, mem::MaybeUninit, num::NonZeroUsize, ops::Deref, ptr, slice};
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
pub trait Consumer: Observer {
    /// Provides a direct access to the ring buffer occupied memory.
    /// The difference from [`Self::as_slices`] is that this method provides slices of [`MaybeUninit`], so items may be moved out of slices.  
    ///
    /// Returns a pair of slices of stored items, the second one may be empty.
    /// Elements with lower indices in slice are older. First slice contains older items that second one.
    ///
    /// # Safety
    ///
    /// All items are initialized. Elements must be removed starting from the beginning of first slice.
    /// When all items are removed from the first slice then items must be removed from the beginning of the second slice.
    ///
    /// *This method must be followed by [`Self::advance_read`] call with the number of items being removed previously as argument.*
    /// *No other mutating calls allowed before that.*
    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]);

    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// Same as [`Self::occupied_slices`].
    ///
    /// # Safety
    ///
    /// When some item is replaced with uninitialized value then it must immediately consumed by [`Self::advance_read`].
    unsafe fn occupied_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    );

    /// Moves `read` pointer by `count` places forward.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied memory must be moved out or dropped.
    unsafe fn advance_read(&mut self, count: usize);

    /// Returns a pair of slices which contain, in order, the contents of the ring buffer.
    #[inline]
    fn as_slices(&self) -> (&[Self::Item], &[Self::Item]) {
        unsafe {
            let (left, right) = self.occupied_slices();
            (slice_assume_init_ref(left), slice_assume_init_ref(right))
        }
    }

    /// Returns a pair of mutable slices which contain, in order, the contents of the ring buffer.
    #[inline]
    fn as_mut_slices(&mut self) -> (&mut [Self::Item], &mut [Self::Item]) {
        unsafe {
            let (left, right) = self.occupied_slices_mut();
            (slice_assume_init_mut(left), slice_assume_init_mut(right))
        }
    }

    /// Removes latest item from the ring buffer and returns it.
    ///
    /// Returns `None` if the ring buffer is empty.
    fn try_pop(&mut self) -> Option<Self::Item> {
        if !self.is_empty() {
            let elem = unsafe { self.occupied_slices().0.get_unchecked(0).assume_init_read() };
            unsafe { self.advance_read(1) };
            Some(elem)
        } else {
            None
        }
    }

    /// Removes items from the ring buffer and writes them into a slice.
    ///
    /// Returns count of items been removed.
    fn pop_slice(&mut self, elems: &mut [Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        let (left, right) = self.occupied_slices();
        let count = if elems.len() < left.len() {
            unsafe { write_uninit_slice(elems, left.get_unchecked(..elems.len())) };
            elems.len()
        } else {
            let (left_elems, elems) = elems.split_at_mut(left.len());
            unsafe { write_uninit_slice(left_elems, left) };
            left.len()
                + if elems.len() < right.len() {
                    unsafe { write_uninit_slice(elems, right.get_unchecked(..elems.len())) };
                    elems.len()
                } else {
                    unsafe { write_uninit_slice(elems.get_unchecked_mut(..right.len()), right) };
                    right.len()
                }
        };
        unsafe { self.advance_read(count) };
        count
    }

    /// Returns an iterator that removes items one by one from the ring buffer.
    fn pop_iter(&mut self) -> PopIter<'_, Self> {
        PopIter(self)
    }

    /// Returns a front-to-back iterator containing references to items in the ring buffer.
    ///
    /// This iterator does not remove items out of the ring buffer.
    fn iter(&self) -> Iter<'_, Self> {
        let (left, right) = self.as_slices();
        left.iter().chain(right.iter())
    }

    /// Returns a front-to-back iterator that returns mutable references to items in the ring buffer.
    ///
    /// This iterator does not remove items out of the ring buffer.
    fn iter_mut(&mut self) -> IterMut<'_, Self> {
        let (left, right) = self.as_mut_slices();
        left.iter_mut().chain(right.iter_mut())
    }

    /// Removes at most `count` and at least `min(count, Self::len())` items from the buffer and safely drops them.
    ///
    /// If there is no concurring producer activity then exactly `min(count, Self::len())` items are removed.
    ///
    /// Returns the number of deleted items.
    ///
    #[cfg_attr(
        feature = "alloc",
        doc = r##"
```
# extern crate ringbuf;
# use ringbuf::{LocalRb, storage::Static, prelude::*};
# fn main() {
let mut rb = LocalRb::<Static<i32, 8>>::default();

assert_eq!(rb.push_iter(&mut (0..8)), 8);

assert_eq!(rb.skip(4), 4);
assert_eq!(rb.skip(8), 4);
assert_eq!(rb.skip(4), 0);
# }
```
"##
    )]
    fn skip(&mut self, count: usize) -> usize {
        unsafe {
            let (left, right) = self.occupied_slices_mut();
            for elem in left.iter_mut().chain(right.iter_mut()).take(count) {
                ptr::drop_in_place(elem.as_mut_ptr());
            }
            let actual_count = usize::min(count, left.len() + right.len());
            self.advance_read(actual_count);
            actual_count
        }
    }

    /// Removes all items from the buffer and safely drops them.
    ///
    /// Returns the number of deleted items.
    fn clear(&mut self) -> usize {
        unsafe {
            let (left, right) = self.occupied_slices_mut();
            for elem in left.iter_mut().chain(right.iter_mut()) {
                ptr::drop_in_place(elem.as_mut_ptr());
            }
            let count = left.len() + right.len();
            self.advance_read(count);
            count
        }
    }
}

pub struct IntoIter<C: Consumer>(C);
impl<C: Consumer> IntoIter<C> {
    pub fn into_inner(self) -> C {
        self.0
    }
}
impl<C: Consumer> Iterator for IntoIter<C> {
    type Item = C::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.try_pop()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.occupied_len(), None)
    }
}

/// An iterator that removes items from the ring buffer.
pub struct PopIter<'a, C: Consumer + ?Sized>(&'a mut C);
impl<'a, C: Consumer + ?Sized> Iterator for PopIter<'a, C> {
    type Item = C::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.try_pop()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.occupied_len(), None)
    }
}

/// Iterator over ring buffer contents.
///
/// *Please do not rely on actual type, it may change in future.*
#[allow(type_alias_bounds)]
pub type Iter<'a, C: Consumer + ?Sized> = Chain<slice::Iter<'a, C::Item>, slice::Iter<'a, C::Item>>;

/// Mutable iterator over ring buffer contents.
///
/// *Please do not rely on actual type, it may change in future.*
#[allow(type_alias_bounds)]
pub type IterMut<'a, C: Consumer + ?Sized> =
    Chain<slice::IterMut<'a, C::Item>, slice::IterMut<'a, C::Item>>;

/// Producer wrapper of ring buffer.
pub struct Cons<R: Deref>
where
    R::Target: RawBase,
{
    raw: R,
}

impl<R: Deref> Cons<R>
where
    R::Target: RawBase,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(raw: R) -> Self {
        Self { raw }
    }
}

impl<R: Deref> RawBase for Cons<R>
where
    R::Target: RawBase,
{
    type Item = <R::Target as RawBase>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.raw.capacity()
    }
    #[inline]
    unsafe fn slice(&self, range: core::ops::Range<usize>) -> &mut [MaybeUninit<Self::Item>] {
        self.raw.slice(range)
    }
    #[inline]
    fn read_end(&self) -> usize {
        self.raw.read_end()
    }
    #[inline]
    fn write_end(&self) -> usize {
        self.raw.write_end()
    }
}

impl<R: Deref> RawCons for Cons<R>
where
    R::Target: RawCons,
{
    #[inline]
    unsafe fn set_read_end(&self, value: usize) {
        self.raw.set_read_end(value)
    }
}

impl<R: Deref> IntoIterator for Cons<R>
where
    R::Target: RawCons,
{
    type Item = <R::Target as RawBase>::Item;
    type IntoIter = IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

#[cfg(feature = "std")]
impl<R: Deref> Cons<R>
where
    R::Target: RawCons<Item = u8>,
{
    #[cfg(feature = "std")]
    /// Removes at most first `count` bytes from the ring buffer and writes them into a [`Write`] instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    ///
    /// Returns `Ok(n)` if `write` succeeded. `n` is number of bytes been written.
    /// `n == 0` means that either `write` returned zero or ring buffer is empty.
    ///
    /// If `write` is failed then original error is returned. In this case it is guaranteed that no items was written to the writer.
    /// To achieve this we write only one contiguous slice at once. So this call may write less than `len` items even if the writer is ready to get more.
    pub fn write_into<S: Write>(
        &mut self,
        writer: &mut S,
        count: Option<usize>,
    ) -> io::Result<usize> {
        let (left, _) = self.occupied_slices();
        let count = usize::min(count.unwrap_or(left.len()), left.len());
        let left_init = unsafe { slice_assume_init_ref(&left[..count]) };

        let write_count = writer.write(left_init)?;
        assert!(write_count <= count);
        unsafe { self.advance_read(write_count) };
        Ok(write_count)
    }
}

#[cfg(feature = "std")]
impl<R: Deref> Read for Cons<R>
where
    R::Target: RawCons<Item = u8>,
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
