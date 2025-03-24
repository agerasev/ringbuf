use crate::{
    rb::AsyncRb,
    wrap::{AsyncCons, AsyncProd},
};
use ringbuf::{storage::Array, SharedRb};
#[cfg(feature = "alloc")]
use ringbuf::{storage::Heap, HeapRb};

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

#[cfg(feature = "alloc")]
impl<T> AsyncHeapRb<T> {
    pub fn new(cap: usize) -> Self {
        Self::from(HeapRb::new(cap))
    }
}

pub type AsyncStaticRb<T, const N: usize> = AsyncRb<Array<T, N>>;
pub type AsyncStaticProd<'a, T, const N: usize> = AsyncProd<&'a AsyncStaticRb<T, N>>;
pub type AsyncStaticCons<'a, T, const N: usize> = AsyncCons<&'a AsyncStaticRb<T, N>>;

impl<T, const N: usize> Default for AsyncRb<Array<T, N>> {
    fn default() -> Self {
        AsyncRb::from(SharedRb::default())
    }
}
