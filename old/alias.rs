use crate::{Consumer, Producer, SharedRb};
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

/// Stack-allocated ring buffer with static capacity.
///
/// *Capacity (`N`) must be greater that zero.*
pub type StaticRb<T, const N: usize> = SharedRb<T, [MaybeUninit<T>; N]>;
/// Alias for [`StaticRb`] [`Producer`].
pub type StaticProducer<'a, T, const N: usize> = Producer<T, &'a StaticRb<T, N>>;
/// Alias for [`StaticRb`] [`Consumer`].
pub type StaticConsumer<'a, T, const N: usize> = Consumer<T, &'a StaticRb<T, N>>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRb<T> = SharedRb<T, Vec<MaybeUninit<T>>>;
/// Alias for [`HeapRb`] [`Producer`].
#[cfg(feature = "alloc")]
pub type HeapProducer<T> = Producer<T, Arc<HeapRb<T>>>;
/// Alias for [`HeapRb`] [`Consumer`].
#[cfg(feature = "alloc")]
pub type HeapConsumer<T> = Consumer<T, Arc<HeapRb<T>>>;
