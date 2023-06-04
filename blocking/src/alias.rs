#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
use crate::{rb::BlockingRb, sync::Semaphore};
#[cfg(feature = "alloc")]
use ringbuf::{storage::Heap, HeapRb};
use ringbuf::{storage::Static, SharedRb};

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
pub type BlockingStaticRb<T, const N: usize, X = StdSemaphore> = BlockingRb<Static<T, N>, X>;
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub type BlockingStaticRb<T, const N: usize, X> = BlockingRb<Static<T, N>, X>;

impl<T, const N: usize, X: Semaphore> Default for BlockingRb<Static<T, N>, X> {
    fn default() -> Self {
        BlockingRb::from(SharedRb::default())
    }
}
