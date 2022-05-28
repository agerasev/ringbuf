use super::SharedRingBuffer;
use crate::utils::uninit_array;
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

impl<T, const N: usize> Default for SharedRingBuffer<T, [MaybeUninit<T>; N]> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(uninit_array(), 0, 0) }
    }
}

#[cfg(feature = "alloc")]
impl<T> SharedRingBuffer<T, Vec<MaybeUninit<T>>> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}

/// Stack-allocated ring buffer with static capacity.
///
/// *Capacity (`N`) must be greater that zero.*
pub type StaticRingBuffer<T, const N: usize> = SharedRingBuffer<T, [MaybeUninit<T>; N]>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRingBuffer<T> = SharedRingBuffer<T, Vec<MaybeUninit<T>>>;
