#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::{cell::UnsafeCell, mem::MaybeUninit, num::NonZeroUsize, ops::Range, ptr::NonNull, slice};

/// Abstract storage for the ring buffer.
///
/// Storage items must be stored as a contiguous array.
///
/// Storage is converted to internal representation before use (see [`Self::Internal`]).
///
/// # Safety
///
/// *[`Self::len`] must always return the same value.*
pub unsafe trait Storage {
    /// Stored item.
    type Item: Sized;

    /// Internal representation of the storage.
    ///
    /// *Must not be aliased with its content.*
    type Internal: ?Sized;

    /// Transform storage to internal representation.
    fn into_internal(self) -> Self::Internal
    where
        Self: Sized,
        Self::Internal: Sized;
    /// Restore storage from internal representation.
    ///
    /// # Safety
    ///
    /// `this` must be a valid internal representation.
    unsafe fn from_internal(this: Self::Internal) -> Self
    where
        Self: Sized,
        Self::Internal: Sized;

    /// Return pointer to the beginning of the storage items.
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<Self::Item>;

    /// Length of the storage.
    fn len(this: &Self::Internal) -> usize;
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

unsafe impl<T> Storage for [MaybeUninit<T>] {
    type Item = T;

    type Internal = UnsafeCell<[MaybeUninit<T>]>;

    fn into_internal(self) -> Self::Internal
    where
        Self: Sized,
    {
        unreachable!()
    }
    unsafe fn from_internal(_this: Self::Internal) -> Self
    where
        Self: Sized,
    {
        unreachable!()
    }

    #[inline]
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<T> {
        this.get() as *mut _
    }

    #[inline]
    fn len(this: &Self::Internal) -> usize {
        unsafe { NonNull::new_unchecked(this.get()) }.len()
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
}

/// Wrapper for storage that provides multiple write access to it.
pub(crate) struct Shared<S: Storage + ?Sized> {
    internal: S::Internal,
}

unsafe impl<S: Storage> Sync for Shared<S> where S::Item: Send {}

impl<S: Storage + ?Sized> Shared<S> {
    /// Create new storage.
    ///
    /// *Panics if storage is empty.*
    pub fn new(storage: S) -> Self
    where
        S: Sized,
        S::Internal: Sized,
    {
        let internal = storage.into_internal();
        assert!(S::len(&internal) > 0);
        Self { internal }
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
    pub fn into_inner(self) -> S
    where
        S: Sized,
        S::Internal: Sized,
    {
        unsafe { S::from_internal(self.internal) }
    }
}

/// Static storage.
pub type Static<T, const N: usize> = [MaybeUninit<T>; N];
/// Heap storage.
#[cfg(feature = "alloc")]
pub type Heap<T> = Vec<MaybeUninit<T>>;
/// Unsized slice storage.
pub type Slice<T> = [MaybeUninit<T>];
