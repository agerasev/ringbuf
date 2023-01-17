use super::{RbBase, RbRead, RbWrite};
use crate::{
    consumer::PopIterator,
    utils::{slice_assume_init_mut, slice_assume_init_ref},
    Consumer, Producer,
};
use core::{
    iter::Chain,
    ops::{Deref, DerefMut},
    slice,
};

#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

/// An abstract ring buffer.
///
/// See [`RbBase`] for details of internal implementation of the ring buffer.
///
/// This trait contains methods that takes `&mut self` allowing you to use ring buffer without splitting it into [`Producer`] and [`Consumer`].
///
/// There are `push*_overwrite` methods that cannot be used from [`Producer`].
///
/// The ring buffer can be guarded with mutex or other synchronization primitive and be used from different threads without splitting (but now only in blocking mode, obviously).
pub trait Rb<T>: RbRead<T> + RbWrite<T> {
    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    fn capacity(&self) -> usize {
        <Self as RbBase<T>>::capacity_nonzero(self).get()
    }

    /// The number of items stored in the ring buffer.
    fn len(&self) -> usize {
        self.occupied_len()
    }

    /// The number of remaining free places in the buffer.
    #[inline]
    fn free_len(&self) -> usize {
        self.vacant_len()
    }

    /// Returns a pair of slices which contain, in order, the contents of the ring buffer.
    #[inline]
    fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            let (left, right) = self.occupied_slices();
            (slice_assume_init_ref(left), slice_assume_init_ref(right))
        }
    }

    /// Returns a pair of mutable slices which contain, in order, the contents of the ring buffer.
    #[inline]
    fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            let (left, right) = self.occupied_slices();
            (slice_assume_init_mut(left), slice_assume_init_mut(right))
        }
    }

    /// Removes latest item from the ring buffer and returns it.
    ///
    /// Returns `None` if the ring buffer is empty.
    #[inline]
    fn pop(&mut self) -> Option<T> {
        unsafe { Consumer::new(self as &Self) }.pop()
    }

    /// Returns an iterator that removes items one by one from the ring buffer.
    fn pop_iter(&mut self) -> PopIterator<'_, T, RbWrap<Self>> {
        PopIterator::new(unsafe { &*(self as *const Self as *const RbWrap<Self>) })
    }

    /// Returns a front-to-back iterator containing references to items in the ring buffer.
    ///
    /// This iterator does not remove items out of the ring buffer.
    fn iter(&self) -> Chain<slice::Iter<T>, slice::Iter<T>> {
        let (left, right) = self.as_slices();
        left.iter().chain(right.iter())
    }

    /// Returns a front-to-back iterator that returns mutable references to items in the ring buffer.
    ///
    /// This iterator does not remove items out of the ring buffer.
    fn iter_mut(&mut self) -> Chain<slice::IterMut<T>, slice::IterMut<T>> {
        let (left, right) = self.as_mut_slices();
        left.iter_mut().chain(right.iter_mut())
    }

    /// Removes exactly `n` items from the buffer and safely drops them.
    ///
    /// *Panics if `n` is greater than number of items in the ring buffer.*
    fn skip(&mut self, count: usize) -> usize {
        assert!(count <= self.len());
        unsafe { self.skip_internal(Some(count)) };
        count
    }

    /// Removes all items from the buffer and safely drops them.
    ///
    /// Returns the number of deleted items.
    #[inline]
    fn clear(&mut self) -> usize {
        unsafe { self.skip_internal(None) }
    }

    /// Appends an item to the ring buffer.
    ///
    /// On failure returns an `Err` containing the item that hasn't been appended.
    #[inline]
    fn push(&mut self, elem: T) -> Result<(), T> {
        unsafe { Producer::new(self as &Self) }.push(elem)
    }

    /// Pushes an item to the ring buffer overwriting the latest item if the buffer is full.
    ///
    /// Returns overwritten item if overwriting took place.
    fn push_overwrite(&mut self, elem: T) -> Option<T> {
        let ret = if self.is_full() { self.pop() } else { None };
        let _ = self.push(elem);
        ret
    }

    /// Appends items from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    #[inline]
    fn push_iter<I: Iterator<Item = T>>(&mut self, iter: &mut I) {
        unsafe { Producer::new(self as &Self) }.push_iter(iter);
    }

    /// Appends items from an iterator to the ring buffer.
    ///
    /// *This method consumes iterator until its end.*
    /// Exactly last `min(iter.len(), capacity)` items from the iterator will be stored in the ring buffer.
    fn push_iter_overwrite<I: Iterator<Item = T>>(&mut self, iter: I) {
        for elem in iter {
            self.push_overwrite(elem);
        }
    }

    /// Removes first items from the ring buffer and writes them into a slice.
    /// Elements must be [`Copy`].
    ///
    /// *Panics if slice length is greater than number of items in the ring buffer.*
    fn pop_slice(&mut self, elems: &mut [T])
    where
        T: Copy,
    {
        assert!(elems.len() <= self.len());
        let _ = unsafe { Consumer::new(self as &Self) }.pop_slice(elems);
    }

    /// Appends items from slice to the ring buffer.
    /// Elements must be [`Copy`].
    ///
    /// *Panics if slice length is greater than number of free places in the ring buffer.*
    fn push_slice(&mut self, elems: &[T])
    where
        T: Copy,
    {
        assert!(elems.len() <= self.free_len());
        let _ = unsafe { Producer::new(self as &Self) }.push_slice(elems);
    }

    /// Appends items from slice to the ring buffer overwriting existing items in the ring buffer.
    ///
    /// If the slice length is greater than ring buffer capacity then only last `capacity` items from slice will be stored in the buffer.
    fn push_slice_overwrite(&mut self, elems: &[T])
    where
        T: Copy,
    {
        if elems.len() > self.free_len() {
            self.skip(usize::min(elems.len() - self.free_len(), self.len()));
        }
        self.push_slice(if elems.len() > self.free_len() {
            &elems[(elems.len() - self.free_len())..]
        } else {
            elems
        });
    }
}

/// An abstract reference to the ring buffer.
pub trait RbRef: Deref<Target = Self::Rb> {
    type Rb: ?Sized;
}
impl<B: ?Sized> RbRef for RbWrap<B> {
    type Rb = B;
}
impl<'a, B: ?Sized> RbRef for &'a B {
    type Rb = B;
}
#[cfg(feature = "alloc")]
impl<B: ?Sized> RbRef for Rc<B> {
    type Rb = B;
}
#[cfg(feature = "alloc")]
impl<B: ?Sized> RbRef for Arc<B> {
    type Rb = B;
}

/// Just a wrapper for a ring buffer.
///
/// Used to make an owning implementation of [`RbRef`].
#[repr(transparent)]
pub struct RbWrap<B: ?Sized>(pub B);
impl<B: ?Sized> Deref for RbWrap<B> {
    type Target = B;
    fn deref(&self) -> &B {
        &self.0
    }
}
impl<B: ?Sized> DerefMut for RbWrap<B> {
    fn deref_mut(&mut self) -> &mut B {
        &mut self.0
    }
}
