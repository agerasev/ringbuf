#[cfg(feature = "alloc")]
use super::storage::Heap;
use super::{
    halves::{Cons, Prod},
    rb::SharedRb,
    storage::Static,
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Stack-allocated ring buffer with static capacity.
///
/// *Capacity (`N`) must be greater that zero.*
pub type StaticRb<T, const N: usize> = SharedRb<Static<T, N>>;

/// Alias for [`StaticRb`] producer.
pub type StaticProd<'a, T, const N: usize> = Prod<&'a StaticRb<T, N>>;

/// Alias for [`StaticRb`] consumer.
pub type StaticCons<'a, T, const N: usize> = Cons<&'a StaticRb<T, N>>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRb<T> = SharedRb<Heap<T>>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] producer.
pub type HeapProd<T> = Prod<Arc<HeapRb<T>>>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] consumer.
pub type HeapCons<T> = Cons<Arc<HeapRb<T>>>;
