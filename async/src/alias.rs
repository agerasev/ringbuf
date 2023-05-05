use crate::{consumer::AsyncConsumer, producer::AsyncProducer, ring_buffer::AsyncRb};
use alloc::sync::Arc;
use ringbuf::HeapRb;

/// Heap-allocated ring buffer.
pub type AsyncHeapRb<T> = AsyncRb<T, HeapRb<T>>;
/// Alias for [`HeapRb`] [`Producer`].
pub type AsyncHeapProducer<T> = AsyncProducer<T, Arc<AsyncHeapRb<T>>>;
/// Alias for [`HeapRb`] [`Consumer`].
pub type AsyncHeapConsumer<T> = AsyncConsumer<T, Arc<AsyncHeapRb<T>>>;
