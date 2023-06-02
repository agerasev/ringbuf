#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
#[cfg(feature = "alloc")]
use crate::{rb::BlockingRb, sync::Semaphore};
#[cfg(feature = "alloc")]
use ringbuf::HeapRb;

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
