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
use crate::ring_buffer::RingBuffer;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Producer that holds reference to `StaticRingBuffer`.
pub type StaticProducer<'a, T, const N: usize> =
    GlobalProducer<T, StaticRingBuffer<T, N>, &'a StaticRingBuffer<T, N>>;

/// Producer that holds `Arc<RingBuffer>`.
#[cfg(feature = "alloc")]
pub type Producer<T> = GlobalProducer<T, RingBuffer<T>, Arc<RingBuffer<T>>>;
