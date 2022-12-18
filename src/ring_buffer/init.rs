use super::{LocalRb, SharedRb};
use crate::utils::uninit_array;
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

impl<T, const N: usize> Default for LocalRb<T, [MaybeUninit<T>; N]> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(uninit_array(), 0, 0) }
    }
}
impl<T: Send, const N: usize> Default for SharedRb<T, [MaybeUninit<T>; N]> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(uninit_array(), 0, 0) }
    }
}

#[cfg(feature = "alloc")]
impl<T> LocalRb<T, Vec<MaybeUninit<T>>> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}
#[cfg(feature = "alloc")]
impl<T: Send> SharedRb<T, Vec<MaybeUninit<T>>> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}
