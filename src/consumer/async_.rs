use super::GlobalConsumer;
use crate::ring_buffer::{AbstractAsyncRingBuffer, RingBufferRef};

pub struct GlobalAsyncConsumer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    basic: GlobalConsumer<T, B, R>,
}

impl<T, B, R> GlobalAsyncConsumer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            basic: GlobalConsumer::new(ring_buffer),
        }
    }
}
