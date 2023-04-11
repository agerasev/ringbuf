#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ops::Range, slice,
};

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

pub type Static<T, const N: usize> = [MaybeUninit<T>; N];
#[cfg(feature = "alloc")]
pub type Heap<T> = Vec<MaybeUninit<T>>;

macro_rules! impl_rb_ctors {
    ($Rb:ident) => {
        impl<T, const N: usize> Default for $Rb<crate::storage::Static<T, N>> {
            fn default() -> Self {
                unsafe { Self::from_raw_parts(crate::utils::uninit_array(), 0, 0) }
            }
        }

        #[cfg(feature = "alloc")]
        impl<T> $Rb<crate::storage::Heap<T>> {
            /// Creates a new instance of a ring buffer.
            ///
            /// *Panics if allocation failed or `capacity` is zero.*
            pub fn new(capacity: usize) -> Self {
                Self::try_new(capacity).unwrap()
            }
            /// Creates a new instance of a ring buffer returning an error if allocation failed.
            ///
            /// *Panics if `capacity` is zero.*
            pub fn try_new(capacity: usize) -> Result<Self, alloc::collections::TryReserveError> {
                let mut data = alloc::vec::Vec::new();
                data.try_reserve_exact(capacity)?;
                data.resize_with(capacity, core::mem::MaybeUninit::uninit);
                Ok(unsafe { Self::from_raw_parts(data, 0, 0) })
            }
        }
    };
}
pub(crate) use impl_rb_ctors;
