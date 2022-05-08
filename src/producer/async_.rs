use super::Producer;
use crate::ring_buffer::{AbstractAsyncRingBuffer, RingBufferRef};

pub struct AsyncProducer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    basic: Producer<T, B, R>,
}

impl<T, B, R> AsyncProducer<T, B, R>
where
    B: AbstractAsyncRingBuffer<T>,
    R: RingBufferRef<T, B>,
{
    pub unsafe fn new(ring_buffer: R) -> Self {
        Self {
            basic: Producer::new(ring_buffer),
        }
    }
}
