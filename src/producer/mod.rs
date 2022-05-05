mod global;
mod local;

pub use global::*;
pub use local::*;

use crate::ring_buffer::StaticRingBuffer;
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use crate::ring_buffer::RingBuffer;
#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

/// Producer that holds reference to `StaticRingBuffer`.
pub type StaticProducer<'a, T, const N: usize> =
    GlobalProducer<T, [MaybeUninit<T>; N], &'a StaticRingBuffer<T, N>>;

/// Producer that holds `Arc<RingBuffer>`.
#[cfg(feature = "alloc")]
pub type Producer<T> = GlobalProducer<T, Vec<MaybeUninit<T>>, Arc<RingBuffer<T>>>;
