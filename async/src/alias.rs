use crate::{
    halves::{AsyncCons, AsyncProd},
    rb::AsyncRb,
};
use alloc::sync::Arc;
use ringbuf::{CachedCons, CachedProd, HeapRb};

pub type AsyncHeapRb<T> = AsyncRb<HeapRb<T>>;

impl<T> AsyncHeapRb<T> {
    pub fn new(cap: usize) -> Self {
        Self::from(HeapRb::new(cap))
    }
}

pub type AsyncHeapProd<T> = AsyncProd<CachedProd<Arc<AsyncHeapRb<T>>>>;
pub type AsyncHeapCons<T> = AsyncCons<CachedCons<Arc<AsyncHeapRb<T>>>>;
