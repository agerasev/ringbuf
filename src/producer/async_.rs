use super::GlobalProducer;
use crate::ring_buffer::{AbstractAsyncRingBuffer, RingBufferRef};

pub struct GlobalAsyncProducer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    basic: GlobalProducer<T, B, R>,
}

impl<T, B, R> GlobalAsyncProducer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            basic: GlobalProducer::new(ring_buffer),
        }
    }
}
