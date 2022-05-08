#[cfg(feature = "async")]
mod async_;
mod global;
mod local;

#[cfg(feature = "async")]
pub use async_::*;
pub use global::*;
pub use local::*;

use crate::ring_buffer::StaticRingBuffer;

#[cfg(feature = "alloc")]
use crate::ring_buffer::HeapRingBuffer;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Producer that holds reference to `StaticRingBuffer`.
pub type StaticProducer<'a, T, const N: usize> =
    Producer<T, StaticRingBuffer<T, N>, &'a StaticRingBuffer<T, N>>;

/// Producer that holds `Arc<HeapRingBuffer>`.
#[cfg(feature = "alloc")]
pub type HeapProducer<T> = Producer<T, HeapRingBuffer<T>, Arc<HeapRingBuffer<T>>>;
