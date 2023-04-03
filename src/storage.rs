#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ops::Range, slice,
};

use crate::raw::RawStorage;

/// Abstract storage for the ring buffer.
///
/// Storage items must be stored as a contiguous array.
///
/// # Safety
///
/// *[`Self::len`]/[`Self::is_empty`] must always return the same value.*
pub unsafe trait Storage {
    type Item: Sized;

    /// Internal representation of the storage.
    ///
    /// *Must not be aliased with its content.*
    type Internal;

    /// Transform storage to internal representation.
    fn into_internal(self) -> Self::Internal;
    /// Restore storage from internal representation.
    ///
    /// # Safety
    ///
    /// `this` must be a valid internal representation.
    unsafe fn from_internal(this: Self::Internal) -> Self;

    /// Return pointer to the beginning of the storage items.
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<Self::Item>;

    /// Length of the storage.
    fn len(this: &Self::Internal) -> usize;

    fn into_rb<R: StoredRb<Storage = Self>>(self) -> R
    where
        Self: Sized,
    {
        unsafe { R::from_raw_parts(self, 0, 0) }
    }
}

unsafe impl<'a, T> Storage for &'a mut [MaybeUninit<T>] {
    type Item = T;

    type Internal = (*mut MaybeUninit<T>, usize);

    fn into_internal(self) -> Self::Internal {
        (self.as_mut_ptr(), self.len())
    }
    unsafe fn from_internal(this: Self::Internal) -> Self {
        slice::from_raw_parts_mut(this.0, this.1)
    }

    #[inline]
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<T> {
        this.0
    }

    #[inline]
    fn len(this: &Self::Internal) -> usize {
        this.1
    }
}

unsafe impl<T, const N: usize> Storage for [MaybeUninit<T>; N] {
    type Item = T;

    type Internal = UnsafeCell<[MaybeUninit<T>; N]>;

    fn into_internal(self) -> Self::Internal {
        UnsafeCell::new(self)
    }
    unsafe fn from_internal(this: Self::Internal) -> Self {
        this.into_inner()
    }

    #[inline]
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<T> {
        this.get() as *mut _
    }

    #[inline]
    fn len(_: &Self::Internal) -> usize {
        N
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T> Storage for Vec<MaybeUninit<T>> {
    type Item = T;

    type Internal = Self;

    fn into_internal(self) -> Self::Internal {
        self
    }
    unsafe fn from_internal(this: Self::Internal) -> Self {
        this
    }

    #[inline]
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<T> {
        this.as_ptr() as *mut _
    }

    #[inline]
    fn len(this: &Self::Internal) -> usize {
        this.len()
    }

    fn into_rb<R: StoredRb<Storage = Self>>(mut self) -> R
    where
        Self: Sized,
    {
        self.resize_with(self.capacity(), MaybeUninit::uninit);
        unsafe { R::from_raw_parts(self, 0, 0) }
    }
}

/// Wrapper for storage that provides multiple write access to it.
pub struct Shared<S: Storage> {
    internal: S::Internal,
    _p: PhantomData<S::Item>,
}

unsafe impl<S: Storage> Sync for Shared<S> where S::Item: Send {}

impl<S: Storage> Shared<S> {
    /// Create new storage.
    ///
    /// *Panics if storage is empty.*
    pub fn new(storage: S) -> Self {
        let internal = storage.into_internal();
        assert!(S::len(&internal) > 0);
        Self {
            internal,
            _p: PhantomData,
        }
    }

    /// Get total length of the storage.
    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(S::len(&self.internal)) }
    }

    /// Returns a mutable slice of storage in specified `range`.
    ///
    /// # Safety
    ///
    /// Slices with overlapping lifetimes must not overlap.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn slice(&self, range: Range<usize>) -> &mut [MaybeUninit<S::Item>] {
        let ptr = S::as_mut_ptr(&self.internal);
        slice::from_raw_parts_mut(ptr.add(range.start), range.len())
    }

    /// Returns underlying storage.
    pub fn into_inner(self) -> S {
        unsafe { S::from_internal(self.internal) }
    }
}

pub trait StoredRb {
    type Storage: Storage;

    /// Constructs ring buffer from storage and counters.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see [`RbBase`](`crate::ring_buffer::RbBase`)).
    unsafe fn from_raw_parts(storage: Self::Storage, read: usize, write: usize) -> Self;

    /// Destructures ring buffer into underlying storage and `read` and `write` counters.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    unsafe fn into_raw_parts(self) -> (Self::Storage, usize, usize);

    fn storage(&self) -> &Shared<Self::Storage>;
}

impl<R: StoredRb> RawStorage for R {
    type Item = <R::Storage as Storage>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage().len()
    }

    #[inline]
    unsafe fn slice(&self, range: Range<usize>) -> &mut [MaybeUninit<Self::Item>] {
        self.storage().slice(range)
    }
}
