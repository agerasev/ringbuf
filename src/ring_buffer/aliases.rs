use super::OwningRingBuffer;
use crate::{
    counter::{AtomicCounter, DefaultCounter},
    utils::uninit_array,
};
use core::{mem::MaybeUninit, num::NonZeroUsize};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

impl<T, S: DefaultCounter, const N: usize> Default for OwningRingBuffer<T, [MaybeUninit<T>; N], S> {
    fn default() -> Self {
        let counter = S::with_capacity(NonZeroUsize::new(N).unwrap());
        unsafe { Self::from_raw_parts(uninit_array(), counter) }
    }
}

#[cfg(feature = "alloc")]
impl<T, S: DefaultCounter> OwningRingBuffer<T, Vec<MaybeUninit<T>>, S> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        let counter = S::with_capacity(NonZeroUsize::new(capacity).unwrap());
        unsafe { Self::from_raw_parts(data, counter) }
    }
}

/// Stack-allocated ring buffer with static capacity.
///
/// *Capacity (`N`) must be greater that zero.*
pub type StaticRingBuffer<T, const N: usize> =
    OwningRingBuffer<T, [MaybeUninit<T>; N], AtomicCounter>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRingBuffer<T> = OwningRingBuffer<T, Vec<MaybeUninit<T>>, AtomicCounter>;
