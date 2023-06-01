use crate::{
    halves::{BlockingCons, BlockingProd},
    sync::{Semaphore, StdSemaphore},
    traits::{BlockingConsumer, BlockingProducer},
};
use core::time::Duration;
use ringbuf::{
    delegate_observer, delegate_ring_buffer,
    rb::traits::{GenSplit, GenSplitRef},
    traits::{Consumer, Observer, Producer, RingBuffer, Split, SplitRef},
};

#[cfg(not(feature = "std"))]
#[derive(Default)]
pub struct BlockingRb<B: RingBuffer, S: Semaphore> {
    base: B,
    read: S,
    write: S,
}
#[cfg(feature = "std")]
#[derive(Default)]
pub struct BlockingRb<B: RingBuffer, S: Semaphore = StdSemaphore> {
    base: B,
    read: S,
    write: S,
}

impl<B: RingBuffer, S: Semaphore> BlockingRb<B, S> {
    pub fn from(base: B) -> Self {
        Self {
            base,
            read: S::default(),
            write: S::default(),
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

impl<'a, B: RingBuffer + GenSplitRef<'a, Self> + 'a, S: Semaphore + 'a> SplitRef<'a> for BlockingRb<B, S> {
    type RefProd = BlockingProd<B::GenRefProd>;
    type RefCons = BlockingCons<B::GenRefCons>;

    fn split_ref(&'a mut self) -> (Self::RefProd, Self::RefCons) {
        let (prod, cons) = B::gen_split_ref(self);
        (BlockingProd::new(prod), BlockingCons::new(cons))
    }
}
impl<B: RingBuffer + GenSplit<Self>, S: Semaphore> Split for BlockingRb<B, S> {
    type Prod = BlockingProd<B::GenProd>;
    type Cons = BlockingCons<B::GenCons>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let (prod, cons) = B::gen_split(self);
        (BlockingProd::new(prod), BlockingCons::new(cons))
    }
}
