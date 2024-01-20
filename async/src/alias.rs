use crate::{
    rb::AsyncRb,
    wrap::{AsyncCons, AsyncProd},
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use ringbuf::{storage::Array, SharedRb};
#[cfg(feature = "alloc")]
use ringbuf::{storage::Heap, HeapRb};

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
