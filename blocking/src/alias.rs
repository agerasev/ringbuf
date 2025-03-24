#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
use crate::{rb::BlockingRb, sync::Semaphore};
use ringbuf::{storage::Array, SharedRb};
#[cfg(feature = "alloc")]
use ringbuf::{storage::Heap, HeapRb};

#[cfg(all(feature = "alloc", not(feature = "portable-atomic")))]
pub use alloc::sync::Arc;
#[cfg(all(feature = "alloc", feature = "portable-atomic"))]
pub use portable_atomic_util::Arc;

#[cfg(feature = "std")]
pub type BlockingHeapRb<T, X = StdSemaphore> = BlockingRb<Heap<T>, X>;
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub type BlockingHeapRb<T, X> = BlockingRb<Heap<T>, X>;

#[cfg(feature = "alloc")]
impl<T, X: Semaphore> BlockingHeapRb<T, X> {
    pub fn new(cap: usize) -> Self {
        Self::from(HeapRb::new(cap))
    }
}

#[cfg(feature = "std")]
pub type BlockingStaticRb<T, const N: usize, X = StdSemaphore> = BlockingRb<Array<T, N>, X>;
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub type BlockingStaticRb<T, const N: usize, X> = BlockingRb<Array<T, N>, X>;

impl<T, const N: usize, X: Semaphore> Default for BlockingRb<Array<T, N>, X> {
    fn default() -> Self {
        BlockingRb::from(SharedRb::default())
    }
}
