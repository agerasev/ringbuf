use crate::rb::BlockingRb;
#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
use ringbuf::storage::Array;
#[cfg(feature = "alloc")]
use ringbuf::storage::Heap;

#[cfg(all(feature = "alloc", not(feature = "portable-atomic")))]
pub use alloc::sync::Arc;
#[cfg(all(feature = "alloc", feature = "portable-atomic"))]
pub use portable_atomic_util::Arc;

#[cfg(feature = "std")]
pub type BlockingHeapRb<T, X = StdSemaphore> = BlockingRb<Heap<T>, X>;
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub type BlockingHeapRb<T, X> = BlockingRb<Heap<T>, X>;

#[cfg(feature = "std")]
pub type BlockingStaticRb<T, const N: usize, X = StdSemaphore> = BlockingRb<Array<T, N>, X>;
#[cfg(not(feature = "std"))]
pub type BlockingStaticRb<T, const N: usize, X> = BlockingRb<Array<T, N>, X>;

#[cfg(feature = "std")]
pub type BlockingArrayRb<T, const N: usize, X = StdSemaphore> = BlockingRb<Array<T, N>, X>;
#[cfg(not(feature = "std"))]
pub type BlockingArrayRb<T, const N: usize, X> = BlockingRb<Array<T, N>, X>;
