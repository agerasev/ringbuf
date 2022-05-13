mod global;
mod local;

pub use global::*;
pub use local::*;

use crate::{counter::AtomicCounter, ring_buffer::StaticRingBuffer};
use core::mem::MaybeUninit;

#[cfg(feature = "alloc")]
use crate::ring_buffer::HeapRingBuffer;
#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

pub type StaticConsumer<'a, T, const N: usize> =
    Consumer<T, [MaybeUninit<T>; N], AtomicCounter, &'a StaticRingBuffer<T, N>>;

#[cfg(feature = "alloc")]
pub type HeapConsumer<T> = Consumer<T, Vec<MaybeUninit<T>>, AtomicCounter, Arc<HeapRingBuffer<T>>>;
