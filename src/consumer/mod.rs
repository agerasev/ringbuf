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

pub type StaticConsumer<'a, T, const N: usize> =
    Consumer<T, StaticRingBuffer<T, N>, &'a StaticRingBuffer<T, N>>;

#[cfg(feature = "alloc")]
pub type HeapConsumer<T> = Consumer<T, HeapRingBuffer<T>, Arc<HeapRingBuffer<T>>>;
