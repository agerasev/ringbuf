use crate::{local::LocalRb, storage::Storage, utils::uninit_array};
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use alloc::{collections::TryReserveError, vec::Vec};

impl<T, const N: usize> Default for LocalRb<[MaybeUninit<T>; N]> {
    fn default() -> Self {
        uninit_array().into_rb()
    }
}

#[cfg(feature = "alloc")]
impl<T> LocalRb<Vec<MaybeUninit<T>>> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if allocation failed or `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        Self::try_new(capacity).unwrap()
    }

    /// Creates a new instance of a ring buffer returning an error if allocation failed.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn try_new(capacity: usize) -> Result<Self, TryReserveError> {
        let mut data = Vec::new();
        data.try_reserve_exact(capacity)?;
        Ok(data.into_rb())
    }
}
