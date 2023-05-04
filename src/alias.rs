#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    index::{LocalIndex, SharedIndex},
    ref_::{ArcFamily, RefFamily},
    storage::Static,
    Cons, Prod, Rb,
};

pub type LocalRb<S> = Rb<S, LocalIndex, LocalIndex>;
pub type SharedRb<S> = Rb<S, SharedIndex, SharedIndex>;

/// Stack-allocated ring buffer with static capacity.
///
/// *Capacity (`N`) must be greater that zero.*
pub type StaticRb<T, const N: usize> = SharedRb<Static<T, N>>;

/// Alias for [`StaticRb`] producer.
pub type StaticProd<'a, T, const N: usize> =
    Prod<RefFamily<'a>, Static<T, N>, SharedIndex, SharedIndex>;

/// Alias for [`StaticRb`] consumer.
pub type StaticCons<'a, T, const N: usize> =
    Cons<RefFamily<'a>, Static<T, N>, SharedIndex, SharedIndex>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRb<T> = SharedRb<Heap<T>>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] producer.
pub type HeapProd<T> = Prod<ArcFamily, Heap<T>, SharedIndex, SharedIndex>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] consumer.
pub type HeapCons<T> = Cons<ArcFamily, Heap<T>, SharedIndex, SharedIndex>;
