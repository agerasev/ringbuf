use crate::utils::ring_buffer_ranges;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, slice};

/// Abstract container for the ring buffer.
///
/// Container items must be stored as a contiguous array.
///
/// # Safety
///
/// *[`Self::len`]/[`Self::is_empty`] must always return the same value.*
///
/// *Container must not cause data race on concurrent [`Self::as_mut_slice`]/[`Self::as_mut_ptr`] calls.*
pub unsafe trait Container<T> {
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
    /// `this` must be valid.
    unsafe fn from_internal(this: Self::Internal) -> Self;

    /// Return pointer to the beginning of the container items.
    fn as_mut_ptr(this: &Self::Internal) -> *mut MaybeUninit<T>;
    /// Length of the container.
    fn len(this: &Self::Internal) -> usize;
}

unsafe impl<'a, T> Container<T> for &'a mut [MaybeUninit<T>] {
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

unsafe impl<T, const N: usize> Container<T> for [MaybeUninit<T>; N] {
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
unsafe impl<T> Container<T> for Vec<MaybeUninit<T>> {
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
pub(crate) struct SharedStorage<T, C: Container<T>> {
    container: C::Internal,
    _p: PhantomData<T>,
}

unsafe impl<T, C: Container<T>> Sync for SharedStorage<T, C> where T: Send {}

impl<T, C: Container<T>> SharedStorage<T, C> {
    /// Create new storage.
    ///
    /// *Panics if container is empty.*
    pub fn new(container: C) -> Self {
        let internal = container.into_internal();
        assert!(C::len(&internal) > 0);
        Self {
            container: internal,
            _p: PhantomData,
        }
    }

    /// Get the length of the container.
    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(C::len(&self.container)) }
    }

    /// Returns a pair of slices between `head` and `tail` positions in the storage.
    ///
    /// For more information see [`ring_buffer_ranges`].
    ///
    /// # Safety
    ///
    /// There only single reference to any item allowed to exist at the time.
    pub unsafe fn as_mut_slices(
        &self,
        head: usize,
        tail: usize,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let ranges = ring_buffer_ranges(self.len(), head, tail);
        let ptr = C::as_mut_ptr(&self.container);
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// Returns underlying container.
    pub fn into_inner(self) -> C {
        unsafe { C::from_internal(self.container) }
    }
}
