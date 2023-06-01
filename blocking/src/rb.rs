use crate::{
    sync::Semaphore,
    traits::{BlockingConsumer, BlockingProducer},
};
use core::time::Duration;
use ringbuf::{
    delegate_observer, delegate_ring_buffer,
    traits::{Consumer, Observer, Producer, RingBuffer},
};

/// Blocking read and write modifier for ring buffer.
pub struct BlockingRb<B: RingBuffer, S: Semaphore> {
    base: B,
    read: S,
    write: S,
}

impl<B: RingBuffer, S: Semaphore> BlockingRb<B, S> {
    pub fn new(base: B) -> Self {
        Self {
            base,
            read: S::new(),
            write: S::new(),
        }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}

impl<B: RingBuffer, S: Semaphore> Observer for BlockingRb<B, S> {
    delegate_observer!(B, Self::base);
}
impl<B: RingBuffer, S: Semaphore> Producer for BlockingRb<B, S> {
    unsafe fn set_write_index(&self, value: usize) {
        self.write.notify(|| self.base.set_write_index(value));
    }
}
impl<B: RingBuffer, S: Semaphore> Consumer for BlockingRb<B, S> {
    unsafe fn set_read_index(&self, value: usize) {
        self.read.notify(|| self.base.set_read_index(value));
    }
}
impl<B: RingBuffer, S: Semaphore> RingBuffer for BlockingRb<B, S> {
    delegate_ring_buffer!(Self::base, Self::base_mut);
}

impl<B: RingBuffer, S: Semaphore> BlockingProducer for BlockingRb<B, S> {
    type Instant = S::Instant;
    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.read.wait(|| self.vacant_len() >= count, timeout)
    }
}
impl<B: RingBuffer, S: Semaphore> BlockingConsumer for BlockingRb<B, S> {
    type Instant = S::Instant;
    fn wait_occupied(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.write.wait(|| self.occupied_len() >= count, timeout)
    }
}
