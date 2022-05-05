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

pub type StaticConsumer<'a, T, const N: usize> =
    GlobalConsumer<T, [MaybeUninit<T>; N], &'a StaticRingBuffer<T, N>>;

#[cfg(feature = "alloc")]
pub type Consumer<T> = GlobalConsumer<T, Vec<MaybeUninit<T>>, Arc<RingBuffer<T>>>;
