use super::{
    Raw,
    consumer::Consumer,
    consumer::{Iter, IterMut, PopIter},
    producer::Producer,
    utils::{add_mod, modulus},
};
use crate::utils::{slice_assume_init_mut, slice_assume_init_ref};
use core::mem::MaybeUninit;

/// A backing ring buffer whose data can be accessed through an exclusive borrow.
///
/// Shared references expose only metadata and unsafe raw operations. Use an
/// endpoint for shared-reference data views, or [`Self::as_exclusive`] to pass a
/// standalone buffer to generic producer/consumer algorithms.
pub trait RingBuffer: Raw {
    /// Exclusive access to both roles for use with generic endpoint algorithms.
    #[inline]
    fn as_exclusive(&mut self) -> crate::rb::Exclusive<'_, Self> {
        crate::rb::Exclusive::new(self)
    }

    /// Exclusive-borrow version of [`Consumer::occupied_slices`].
    #[inline]
    fn occupied_slices(&mut self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.unsafe_slices(self.read_index(), self.write_index()) }
    }

    /// Exclusive-borrow version of [`Consumer::as_slices`].
    #[inline]
    fn as_slices(&mut self) -> (&[Self::Item], &[Self::Item]) {
        let (left, right) = self.occupied_slices();
        unsafe { (slice_assume_init_ref(left), slice_assume_init_ref(right)) }
    }

    /// See [`Consumer::as_mut_slices`].
    #[inline]
    fn as_mut_slices(&mut self) -> (&mut [Self::Item], &mut [Self::Item]) {
        let (left, right) = unsafe { self.occupied_slices_mut() };
        unsafe { (slice_assume_init_mut(left), slice_assume_init_mut(right)) }
    }

    /// Exclusive-borrow version of [`Consumer::first`].
    #[inline]
    fn first(&mut self) -> Option<&Self::Item> {
        self.as_slices().0.first()
    }

    /// See [`Consumer::first_mut`].
    #[inline]
    fn first_mut(&mut self) -> Option<&mut Self::Item> {
        self.as_mut_slices().0.first_mut()
    }

    /// Exclusive-borrow version of [`Consumer::last`].
    #[inline]
    fn last(&mut self) -> Option<&Self::Item> {
        let (left, right) = self.as_slices();
        right.last().or_else(|| left.last())
    }

    /// See [`Consumer::last_mut`].
    #[inline]
    fn last_mut(&mut self) -> Option<&mut Self::Item> {
        let (left, right) = self.as_mut_slices();
        right.last_mut().or_else(|| left.last_mut())
    }

    /// Exclusive-borrow version of [`Consumer::try_peek`].
    #[inline]
    fn try_peek(&mut self) -> Option<&Self::Item> {
        self.first()
    }

    /// See [`Consumer::try_pop`].
    #[inline]
    fn try_pop(&mut self) -> Option<Self::Item> {
        self.as_exclusive().try_pop()
    }

    /// Exclusive-borrow version of [`Consumer::peek_slice_uninit`].
    #[inline]
    fn peek_slice_uninit(&mut self, elems: &mut [MaybeUninit<Self::Item>]) -> usize {
        self.as_exclusive().peek_slice_uninit(elems)
    }

    /// Exclusive-borrow version of [`Consumer::peek_slice`].
    #[inline]
    fn peek_slice(&mut self, elems: &mut [Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.as_exclusive().peek_slice(elems)
    }

    /// See [`Consumer::pop_slice_uninit`].
    #[inline]
    fn pop_slice_uninit(&mut self, elems: &mut [MaybeUninit<Self::Item>]) -> usize {
        self.as_exclusive().pop_slice_uninit(elems)
    }

    /// See [`Consumer::pop_slice`].
    #[inline]
    fn pop_slice(&mut self, elems: &mut [Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.as_exclusive().pop_slice(elems)
    }

    /// See [`Consumer::pop_iter`]. Each item is released before it is returned.
    #[inline]
    fn pop_iter(&mut self) -> PopIter<'_, Self> {
        unsafe { PopIter::from_raw(self) }
    }

    /// Exclusive-borrow version of [`Consumer::iter`].
    #[inline]
    fn iter(&mut self) -> Iter<'_, Self> {
        let (left, right) = self.as_slices();
        left.iter().chain(right)
    }

    /// See [`Consumer::iter_mut`].
    #[inline]
    fn iter_mut(&mut self) -> IterMut<'_, Self> {
        let (left, right) = self.as_mut_slices();
        left.iter_mut().chain(right)
    }

    /// See [`Consumer::skip`], including destructor-panic and slot-release behavior.
    #[inline]
    fn skip(&mut self, count: usize) -> usize {
        self.as_exclusive().skip(count)
    }

    /// See [`Consumer::clear`].
    #[inline]
    fn clear(&mut self) -> usize {
        self.as_exclusive().clear()
    }

    /// Exclusive-borrow version of [`Producer::vacant_slices`].
    #[inline]
    fn vacant_slices(&mut self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.unsafe_slices(self.write_index(), add_mod(self.read_index(), self.capacity().get(), modulus(self))) }
    }

    /// See [`Producer::vacant_slices_mut`] for initialization and publication requirements.
    #[inline]
    fn vacant_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.unsafe_slices_mut(self.write_index(), add_mod(self.read_index(), self.capacity().get(), modulus(self))) }
    }

    /// See [`Producer::try_push`].
    #[inline]
    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        self.as_exclusive().try_push(elem)
    }

    /// See [`Producer::push_iter`].
    #[inline]
    fn push_iter<I: Iterator<Item = Self::Item>>(&mut self, iter: I) -> usize {
        self.as_exclusive().push_iter(iter)
    }

    /// See [`Producer::push_slice`].
    #[inline]
    fn push_slice(&mut self, elems: &[Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.as_exclusive().push_slice(elems)
    }

    #[cfg(feature = "std")]
    /// See [`Producer::read_from`].
    #[inline]
    fn read_from<Input: std::io::Read>(&mut self, reader: &mut Input, count: Option<usize>) -> Option<std::io::Result<usize>>
    where
        Self: super::Observer<Item = u8>,
    {
        self.as_exclusive().read_from(reader, count)
    }

    #[cfg(feature = "std")]
    /// See [`Consumer::write_into`].
    #[inline]
    fn write_into<Output: std::io::Write>(&mut self, writer: &mut Output, count: Option<usize>) -> Option<std::io::Result<usize>>
    where
        Self: super::Observer<Item = u8>,
    {
        self.as_exclusive().write_into(writer, count)
    }

    /// Pushes an item to the ring buffer overwriting the least recent item if the buffer is full.
    ///
    /// Returns overwritten item if overwriting took place.
    fn push_overwrite(&mut self, elem: Self::Item) -> Option<Self::Item> {
        let ret = if self.is_full() { self.try_pop() } else { None };
        let _ = self.try_push(elem);
        ret
    }

    /// Appends items from an iterator to the ring buffer.
    ///
    /// *This method consumes iterator until its end.*
    /// Exactly last `min(iter.len(), capacity)` items from the iterator will be stored in the ring buffer.
    fn push_iter_overwrite<I: Iterator<Item = Self::Item>>(&mut self, iter: I) {
        for elem in iter {
            self.push_overwrite(elem);
        }
    }

    /// Appends items from slice to the ring buffer overwriting existing items in the ring buffer.
    ///
    /// If the slice length is greater than ring buffer capacity then only last `capacity` items from slice will be stored in the buffer.
    fn push_slice_overwrite(&mut self, elems: &[Self::Item])
    where
        Self::Item: Copy,
    {
        if elems.len() > self.vacant_len() {
            self.skip(usize::min(elems.len() - self.vacant_len(), self.occupied_len()));
        }
        self.push_slice(if elems.len() > self.vacant_len() {
            &elems[(elems.len() - self.vacant_len())..]
        } else {
            elems
        });
    }
}

/// Trait used for delegating exclusive buffer operations.
pub trait DelegateRingBuffer: super::observer::DelegateObserver + super::DelegateRaw
where
    Self::Base: RingBuffer,
{
}
impl<D: DelegateRingBuffer> RingBuffer for D where D::Base: RingBuffer {}
