use crate::{
    delegate_blocking_consumer, delegate_blocking_producer,
    sync::Semaphore,
    traits::{BlockingConsumer, BlockingProducer},
};
use core::time::Duration;
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer, delegate_ring_buffer,
    traits::{Consumer, Observer, Producer, RingBuffer},
};

/// Blocking write modifier for ring buffer.
pub struct BlockingRbWrite<B: RingBuffer, S: Semaphore> {
    sem: S,
    base: B,
}
/// Blocking read modifier for ring buffer.
pub struct BlockingRbRead<B: RingBuffer, S: Semaphore> {
    sem: S,
    base: B,
}

/// Blocking read and write modifier for ring buffer.
pub struct BlockingRb<B: RingBuffer, S: Semaphore> {
    inner: BlockingRbRead<BlockingRbWrite<B, S>, S>,
}

impl<B: RingBuffer, S: Semaphore> BlockingRbWrite<B, S> {
    pub fn new(base: B) -> Self {
        Self {
            sem: Semaphore::new(),
            base,
        }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}
impl<B: RingBuffer, S: Semaphore> BlockingRbRead<B, S> {
    pub fn new(base: B) -> Self {
        Self {
            sem: Semaphore::new(),
            base,
        }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}
impl<B: RingBuffer, S: Semaphore> BlockingRb<B, S> {
    pub fn new(base: B) -> Self {
        Self {
            inner: BlockingRbRead::new(BlockingRbWrite::new(base)),
        }
    }
    fn base(&self) -> &BlockingRbRead<BlockingRbWrite<B, S>, S> {
        &self.inner
    }
    fn base_mut(&mut self) -> &mut BlockingRbRead<BlockingRbWrite<B, S>, S> {
        &mut self.inner
    }
}

impl<B: RingBuffer, S: Semaphore> Observer for BlockingRbWrite<B, S> {
    delegate_observer!(B, Self::base);
}
impl<B: RingBuffer, S: Semaphore> Producer for BlockingRbWrite<B, S> {
    delegate_producer!(Self::base, Self::base_mut);
}
impl<B: RingBuffer, S: Semaphore> Consumer for BlockingRbWrite<B, S> {
    unsafe fn set_read_index(&self, value: usize) {
        self.sem.notify(|| self.base.set_read_index(value));
    }
}
impl<B: RingBuffer, S: Semaphore> RingBuffer for BlockingRbWrite<B, S> {
    delegate_ring_buffer!(Self::base, Self::base_mut);
}

impl<B: RingBuffer, S: Semaphore> BlockingProducer for BlockingRbWrite<B, S> {
    type Instant = S::Instant;
    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.sem.wait(|| self.vacant_len() >= count, timeout)
    }
}

/// TODO: Use inter-crate delegating mechanism.
impl<B: RingBuffer + BlockingConsumer, S: Semaphore> BlockingConsumer for BlockingRbWrite<B, S> {
    delegate_blocking_consumer!(B, Self::base, Self::base_mut);
}

impl<B: RingBuffer, S: Semaphore> Observer for BlockingRbRead<B, S> {
    delegate_observer!(B, Self::base);
}
impl<B: RingBuffer, S: Semaphore> Producer for BlockingRbRead<B, S> {
    unsafe fn set_write_index(&self, value: usize) {
        self.sem.notify(|| self.base.set_write_index(value));
    }
}
impl<B: RingBuffer, S: Semaphore> Consumer for BlockingRbRead<B, S> {
    delegate_consumer!(Self::base, Self::base_mut);
}
impl<B: RingBuffer, S: Semaphore> RingBuffer for BlockingRbRead<B, S> {
    delegate_ring_buffer!(Self::base, Self::base_mut);
}

impl<B: RingBuffer, S: Semaphore> BlockingConsumer for BlockingRbRead<B, S> {
    type Instant = S::Instant;
    fn wait_occupied(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.sem.wait(|| self.occupied_len() >= count, timeout)
    }
}

/// TODO: Use inter-crate delegating mechanism.
impl<B: RingBuffer + BlockingProducer, S: Semaphore> BlockingProducer for BlockingRbRead<B, S> {
    delegate_blocking_producer!(B, Self::base, Self::base_mut);
}

impl<B: RingBuffer, S: Semaphore> Observer for BlockingRb<B, S> {
    delegate_observer!(B, Self::base);
}
impl<B: RingBuffer, S: Semaphore> Producer for BlockingRb<B, S> {
    delegate_producer!(Self::base, Self::base_mut);
}
impl<B: RingBuffer, S: Semaphore> Consumer for BlockingRb<B, S> {
    delegate_consumer!(Self::base, Self::base_mut);
}
impl<B: RingBuffer, S: Semaphore> RingBuffer for BlockingRb<B, S> {
    delegate_ring_buffer!(Self::base, Self::base_mut);
}

impl<B: RingBuffer, S: Semaphore> BlockingProducer for BlockingRb<B, S> {
    delegate_blocking_producer!(BlockingRbRead<BlockingRbWrite<B, S>, S>, Self::base, Self::base_mut);
}
impl<B: RingBuffer, S: Semaphore> BlockingConsumer for BlockingRb<B, S> {
    delegate_blocking_consumer!(BlockingRbRead<BlockingRbWrite<B, S>, S>, Self::base, Self::base_mut);
}
