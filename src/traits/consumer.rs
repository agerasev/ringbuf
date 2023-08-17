use super::{
    observer::{DelegateObserver, Observer},
    utils::modulus,
};
use crate::utils::{slice_assume_init_mut, slice_assume_init_ref, write_uninit_slice};
use core::{iter::Chain, mem::MaybeUninit, ptr, slice};
#[cfg(feature = "std")]
use std::io::{self, Write};

/// Consumer part of ring buffer.
pub trait Consumer: Observer {
    unsafe fn set_read_index(&self, value: usize);

    /// Moves `read` pointer by `count` places forward.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied memory must be moved out or dropped.
    ///
    /// Must not be called concurrently.
    unsafe fn advance_read_index(&self, count: usize) {
        self.set_read_index((self.read_index() + count) % modulus(self));
    }

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
    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let (first, second) = unsafe { self.unsafe_slices(self.read_index(), self.write_index()) };
        (first as &_, second as &_)
    }

    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// Same as [`Self::occupied_slices`].
    ///
    /// # Safety
    ///
    /// When some item is replaced with uninitialized value then it must not be read anymore.
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.unsafe_slices(self.read_index(), self.write_index())
    }

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
            unsafe { self.advance_read_index(1) };
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
        unsafe { self.advance_read_index(count) };
        count
    }

    fn into_iter(self) -> IntoIter<Self> {
        IntoIter::new(self)
    }

    /// Returns an iterator that removes items one by one from the ring buffer.
    fn pop_iter(&mut self) -> PopIter<'_, Self> {
        PopIter::new(self)
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
    /// ```
    /// # extern crate ringbuf;
    /// # use ringbuf::{LocalRb, storage::Static, traits::*};
    /// # fn main() {
    /// let mut rb = LocalRb::<Static<i32, 8>>::default();
    ///
    /// assert_eq!(rb.push_iter(0..8), 8);
    ///
    /// assert_eq!(rb.skip(4), 4);
    /// assert_eq!(rb.skip(8), 4);
    /// assert_eq!(rb.skip(4), 0);
    /// # }
    /// ```
    fn skip(&mut self, count: usize) -> usize {
        unsafe {
            let (left, right) = self.occupied_slices_mut();
            for elem in left.iter_mut().chain(right.iter_mut()).take(count) {
                ptr::drop_in_place(elem.as_mut_ptr());
            }
            let actual_count = usize::min(count, left.len() + right.len());
            self.advance_read_index(actual_count);
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
            self.advance_read_index(count);
            count
        }
    }

    #[cfg(feature = "std")]
    /// Removes at most first `count` bytes from the ring buffer and writes them into a [`Write`] instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    ///
    /// Returns `Ok(n)` if `write` succeeded. `n` is number of bytes been written.
    /// `n == 0` means that either `write` returned zero or ring buffer is empty.
    ///
    /// If `write` is failed then original error is returned. In this case it is guaranteed that no items was written to the writer.
    /// To achieve this we write only one contiguous slice at once. So this call may write less than `len` items even if the writer is ready to get more.
    fn write_into<S: Write>(&mut self, writer: &mut S, count: Option<usize>) -> io::Result<usize>
    where
        Self: Consumer<Item = u8>,
    {
        let (left, _) = self.occupied_slices();
        let count = usize::min(count.unwrap_or(left.len()), left.len());
        let left_init = unsafe { slice_assume_init_ref(&left[..count]) };

        let write_count = writer.write(left_init)?;
        assert!(write_count <= count);
        unsafe { self.advance_read_index(write_count) };
        Ok(write_count)
    }

    #[cfg(feature = "std")]
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize>
    where
        Self: Consumer<Item = u8>,
    {
        let n = self.pop_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(std::io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }
}

pub struct IntoIter<C: Consumer>(C);
impl<C: Consumer> IntoIter<C> {
    pub fn new(inner: C) -> Self {
        Self(inner)
    }
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
pub struct PopIter<'a, C: Consumer> {
    target: &'a C,
    slices: (&'a [MaybeUninit<C::Item>], &'a [MaybeUninit<C::Item>]),
    len: usize,
}
impl<'a, C: Consumer> PopIter<'a, C> {
    pub fn new(target: &'a mut C) -> Self {
        let slices = target.occupied_slices();
        Self {
            len: slices.0.len() + slices.1.len(),
            slices,
            target,
        }
    }
}
impl<'a, C: Consumer> Iterator for PopIter<'a, C> {
    type Item = C::Item;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
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
impl<'a, C: Consumer> ExactSizeIterator for PopIter<'a, C> {
    fn len(&self) -> usize {
        self.slices.0.len() + self.slices.1.len()
    }
}
impl<'a, C: Consumer> Drop for PopIter<'a, C> {
    fn drop(&mut self) {
        unsafe { self.target.advance_read_index(self.len - self.len()) };
    }
}

/// Iterator over ring buffer contents.
///
/// *Please do not rely on actual type, it may change in future.*
#[allow(type_alias_bounds)]
pub type Iter<'a, C: Consumer> = Chain<slice::Iter<'a, C::Item>, slice::Iter<'a, C::Item>>;

/// Mutable iterator over ring buffer contents.
///
/// *Please do not rely on actual type, it may change in future.*
#[allow(type_alias_bounds)]
pub type IterMut<'a, C: Consumer> = Chain<slice::IterMut<'a, C::Item>, slice::IterMut<'a, C::Item>>;

pub trait DelegateConsumer: DelegateObserver
where
    Self::Delegate: Consumer,
{
}
impl<D: DelegateConsumer> Consumer for D
where
    D::Delegate: Consumer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.delegate().set_read_index(value)
    }
    #[inline]
    unsafe fn advance_read_index(&self, count: usize) {
        self.delegate().advance_read_index(count)
    }

    #[inline]
    fn occupied_slices(&self) -> (&[core::mem::MaybeUninit<Self::Item>], &[core::mem::MaybeUninit<Self::Item>]) {
        self.delegate().occupied_slices()
    }

    #[inline]
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [core::mem::MaybeUninit<Self::Item>], &mut [core::mem::MaybeUninit<Self::Item>]) {
        self.delegate_mut().occupied_slices_mut()
    }

    #[inline]
    fn as_slices(&self) -> (&[Self::Item], &[Self::Item]) {
        self.delegate().as_slices()
    }

    #[inline]
    fn as_mut_slices(&mut self) -> (&mut [Self::Item], &mut [Self::Item]) {
        self.delegate_mut().as_mut_slices()
    }

    #[inline]
    fn try_pop(&mut self) -> Option<Self::Item> {
        self.delegate_mut().try_pop()
    }

    #[inline]
    fn pop_slice(&mut self, elems: &mut [Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        self.delegate_mut().pop_slice(elems)
    }

    #[inline]
    fn iter(&self) -> Iter<'_, Self> {
        self.delegate().iter()
    }

    #[inline]
    fn iter_mut(&mut self) -> IterMut<'_, Self> {
        self.delegate_mut().iter_mut()
    }

    #[inline]
    fn skip(&mut self, count: usize) -> usize {
        self.delegate_mut().skip(count)
    }

    #[inline]
    fn clear(&mut self) -> usize {
        self.delegate_mut().clear()
    }
}
