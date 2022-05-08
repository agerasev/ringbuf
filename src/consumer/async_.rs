use super::Consumer;
use crate::ring_buffer::{AbstractAsyncRingBuffer, RingBufferRef};

pub struct AsyncConsumer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    basic: Consumer<T, B, R>,
}

impl<T, B, R> AsyncConsumer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            basic: Consumer::new(ring_buffer),
        }
    }
}
