use crate::{
    endpoint::{AsyncCons, AsyncProd},
    rb::AsyncRb,
};
use ringbuf::storage::Array;
#[cfg(feature = "alloc")]
use ringbuf::storage::Heap;

#[cfg(all(feature = "alloc", not(feature = "portable-atomic")))]
pub use alloc::sync::Arc;
#[cfg(all(feature = "alloc", feature = "portable-atomic"))]
pub use portable_atomic_util::Arc;

#[cfg(feature = "alloc")]
pub type AsyncHeapRb<T> = AsyncRb<Heap<T>>;
#[cfg(feature = "alloc")]
pub type AsyncHeapProd<T> = AsyncProd<Arc<AsyncHeapRb<T>>>;
#[cfg(feature = "alloc")]
pub type AsyncHeapCons<T> = AsyncCons<Arc<AsyncHeapRb<T>>>;

pub type AsyncStaticRb<T, const N: usize> = AsyncRb<Array<T, N>>;
pub type AsyncStaticProd<'a, T, const N: usize> = AsyncProd<&'a AsyncStaticRb<T, N>>;
pub type AsyncStaticCons<'a, T, const N: usize> = AsyncCons<&'a AsyncStaticRb<T, N>>;

pub type AsyncArrayRb<T, const N: usize> = AsyncRb<Array<T, N>>;
