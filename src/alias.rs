#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    index::{LocalIndex, SharedIndex},
    storage::Static,
    Cons, Prod, Rb,
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;

pub type LocalRb<S> = Rb<S, LocalIndex, LocalIndex>;
pub type SharedRb<S> = Rb<S, SharedIndex, SharedIndex>;

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
