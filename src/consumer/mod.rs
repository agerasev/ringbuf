mod global;
mod local;
mod traits;

pub use global::*;
pub use local::*;
pub use traits::*;

use crate::ring_buffer::StaticRingBuffer;

#[cfg(feature = "alloc")]
use crate::ring_buffer::HeapRingBuffer;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;

pub type StaticConsumer<'a, T, const N: usize> = Consumer<T, &'a StaticRingBuffer<T, N>>;

#[cfg(feature = "alloc")]
pub type HeapConsumer<T> = Consumer<T, Arc<HeapRingBuffer<T>>>;
