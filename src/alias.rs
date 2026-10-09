#[cfg(feature = "alloc")]
use super::storage::Heap;
use super::{
    endpoint::{CachedCons, CachedProd},
    rb::SharedRb,
    storage::Array,
};

#[cfg(all(feature = "alloc", not(feature = "portable-atomic")))]
pub use alloc::sync::Arc;
#[cfg(all(feature = "alloc", feature = "portable-atomic"))]
pub use portable_atomic_util::Arc;

/// Stack-allocated ring buffer with static capacity.
///
/// *Capacity (`N`) must be in `1..=usize::MAX / 2`.*
pub type ArrayRb<T, const N: usize> = SharedRb<Array<T, N>>;

/// Alias for [`ArrayRb`] producer.
pub type ArrayProd<'a, T, const N: usize> = CachedProd<&'a ArrayRb<T, N>>;

/// Alias for [`ArrayRb`] consumer.
pub type ArrayCons<'a, T, const N: usize> = CachedCons<&'a ArrayRb<T, N>>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRb<T> = SharedRb<Heap<T>>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] producer.
pub type HeapProd<T> = CachedProd<Arc<HeapRb<T>>>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] consumer.
pub type HeapCons<T> = CachedCons<Arc<HeapRb<T>>>;

pub type StaticRb<T, const N: usize> = ArrayRb<T, N>;
pub type StaticProd<'a, T, const N: usize> = ArrayProd<'a, T, N>;
pub type StaticCons<'a, T, const N: usize> = ArrayCons<'a, T, N>;
