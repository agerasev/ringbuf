mod global;
mod local;

pub use global::*;
pub use local::*;

use crate::ring_buffer::StaticRingBuffer;

#[cfg(feature = "alloc")]
use crate::ring_buffer::HeapRingBuffer;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Producer that holds reference to `StaticRingBuffer`.
pub type StaticProducer<'a, T, const N: usize> = Producer<T, &'a StaticRingBuffer<T, N>>;

/// Producer that holds `Arc<HeapRingBuffer>`.
#[cfg(feature = "alloc")]
pub type HeapProducer<T> = Producer<T, Arc<HeapRingBuffer<T>>>;
