#[cfg(feature = "async")]
mod async_;
mod basic;
mod counter;
mod storage;

#[cfg(feature = "async")]
pub use async_::*;
pub use basic::*;
pub use counter::*;
pub use storage::*;

use crate::utils::uninit_array;
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Stack-allocated ring buffer with static capacity.
///
/// Capacity must be greater that zero.
pub type StaticRingBuffer<T, const N: usize> = BasicRingBuffer<T, [MaybeUninit<T>; N]>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type RingBuffer<T> = BasicRingBuffer<T, Vec<MaybeUninit<T>>>;

#[cfg(feature = "alloc")]
impl<T> RingBuffer<T> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}

impl<T, const N: usize> Default for StaticRingBuffer<T, N> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(uninit_array(), 0, 0) }
    }
}
