#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    raw::RawBuffer,
    storage::{Shared, Static, Storage},
    utils::uninit_array,
};
#[cfg(feature = "alloc")]
use alloc::{collections::TryReserveError, vec::Vec};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Range};

pub trait StoredRb: Sized {
    type Storage: Storage;

    fn storage(&self) -> &Shared<Self::Storage>;

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
}

impl<R: StoredRb> RawBuffer for R {
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

/// Stack-allocated ring buffer.
pub trait StaticRb: StoredRb {
    fn default() -> Self;
}

#[cfg(feature = "alloc")]
/// Heap-allocated ring buffer.
pub trait HeapRb: StoredRb {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if allocation failed or `capacity` is zero.*
    fn new(capacity: usize) -> Self;

    /// Creates a new instance of a ring buffer returning an error if allocation failed.
    ///
    /// *Panics if `capacity` is zero.*
    fn try_new(capacity: usize) -> Result<Self, TryReserveError>;
}

impl<T, R: StoredRb<Storage = Static<T, N>>, const N: usize> StaticRb for R {
    fn default() -> Self {
        uninit_array().into_rb()
    }
}

#[cfg(feature = "alloc")]
impl<T, R: StoredRb<Storage = Heap<T>>> HeapRb for R {
    fn new(capacity: usize) -> Self {
        Self::try_new(capacity).unwrap()
    }

    fn try_new(capacity: usize) -> Result<Self, TryReserveError> {
        let mut data = Vec::new();
        data.try_reserve_exact(capacity)?;
        Ok(data.into_rb())
    }
}
