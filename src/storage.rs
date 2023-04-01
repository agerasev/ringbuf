#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ops::Range, slice,
};

/// Abstract container for the ring buffer.
///
/// Storage items must be stored as a contiguous array.
///
/// # Safety
///
/// *[`Self::len`]/[`Self::is_empty`] must always return the same value.*
///
/// *Storage must not cause data race on concurrent [`Self::as_mut_slice`]/[`Self::as_mut_ptr`] calls.*
pub unsafe trait Storage {
    type Item: Sized;

    /// Internal representation of the container.
    ///
    /// *Must not be aliased with its content.*
    type Internal;

    /// Transform container to internal representation.
    fn into_internal(self) -> Self::Internal;
    /// Restore container from internal representation.
    ///
    /// # Safety
    ///
    /// `this` must be a valid internal representation.
    unsafe fn from_internal(this: Self::Internal) -> Self;

    /// Return pointer to the beginning of the container items.
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<Self::Item>;

    /// Length of the container.
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

/// Wrapper for container that provides multiple write access to it.
pub(crate) struct SharedStorage<S: Storage> {
    internal: S::Internal,
    _p: PhantomData<S::Item>,
}

unsafe impl<S: Storage> Sync for SharedStorage<S> where S::Item: Send {}

impl<S: Storage> SharedStorage<S> {
    /// Create new storage.
    ///
    /// *Panics if container is empty.*
    pub fn new(container: S) -> Self {
        let internal = container.into_internal();
        assert!(S::len(&internal) > 0);
        Self {
            internal,
            _p: PhantomData,
        }
    }

    /// Get the length of the container.
    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(S::len(&self.internal)) }
    }

    /// Returns a mutable slice of storage in specified `range`.
    ///
    /// # Safety
    ///
    /// There only single reference to any item allowed to exist at the time.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn index_mut(&self, range: Range<usize>) -> &mut [MaybeUninit<S::Item>] {
        let ptr = S::as_mut_ptr(&self.internal);
        slice::from_raw_parts_mut(ptr.add(range.start), range.len())
    }

    /// Returns underlying container.
    pub fn into_inner(self) -> S {
        unsafe { S::from_internal(self.internal) }
    }
}
