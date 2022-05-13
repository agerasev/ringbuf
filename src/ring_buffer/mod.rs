mod self_;
mod storage;

pub use self_::*;
pub use storage::*;

use crate::{counter::AtomicCounter, utils::uninit_array};
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Stack-allocated ring buffer with static capacity.
///
/// Capacity must be greater that zero.
pub type StaticRingBuffer<T, const N: usize> =
    OwningRingBuffer<T, [MaybeUninit<T>; N], AtomicCounter>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRingBuffer<T> = OwningRingBuffer<T, Vec<MaybeUninit<T>>, AtomicCounter>;

impl<T, const N: usize> Default for StaticRingBuffer<T, N> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(uninit_array(), 0, 0) }
    }
}

#[cfg(feature = "alloc")]
impl<T> HeapRingBuffer<T> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}
