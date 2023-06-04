#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
use crate::{rb::BlockingRb, sync::Semaphore, BlockingCons, BlockingProd};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
#[cfg(feature = "alloc")]
use ringbuf::HeapRb;
use ringbuf::{CachedCons, CachedProd};

#[cfg(feature = "std")]
pub type BlockingHeapRb<T, S = StdSemaphore> = BlockingRb<HeapRb<T>, S>;
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub type BlockingHeapRb<T, S> = BlockingRb<HeapRb<T>, S>;

#[cfg(feature = "alloc")]
impl<T, S: Semaphore> BlockingHeapRb<T, S> {
    pub fn new(cap: usize) -> Self {
        Self::from(HeapRb::new(cap))
    }
}

#[cfg(feature = "std")]
pub type BlockingHeapProd<T, S = StdSemaphore> = BlockingProd<CachedProd<Arc<BlockingHeapRb<T, S>>>>;
#[cfg(feature = "std")]
pub type BlockingHeapCons<T, S = StdSemaphore> = BlockingCons<CachedCons<Arc<BlockingHeapRb<T, S>>>>;

#[cfg(all(feature = "alloc", not(feature = "std")))]
pub type BlockingHeapProd<T, S> = BlockingProd<CachedProd<Arc<BlockingHeapRb<T, S>>>>;
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub type BlockingHeapCons<T, S> = BlockingCons<CachedCons<Arc<BlockingHeapRb<T, S>>>>;
