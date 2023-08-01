#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
use crate::{
    halves::{BlockingCons, BlockingProd},
    sync::Semaphore,
    traits::{BlockingConsumer, BlockingProducer},
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use core::time::Duration;
#[cfg(feature = "alloc")]
use ringbuf::traits::Split;
use ringbuf::{
    delegate_observer, delegate_ring_buffer,
    rb::traits::RbRef,
    storage::Storage,
    traits::{Consumer, Observer, Producer, RingBuffer, SplitRef},
    SharedRb,
};

#[cfg(not(feature = "std"))]
pub struct BlockingRb<S: Storage, X: Semaphore> {
    base: SharedRb<S>,
    read: X,
    write: X,
}
#[cfg(feature = "std")]
pub struct BlockingRb<S: Storage, X: Semaphore = StdSemaphore> {
    base: SharedRb<S>,
    read: X,
    write: X,
}

impl<S: Storage, X: Semaphore> BlockingRb<S, X> {
    pub fn from(base: SharedRb<S>) -> Self {
        Self {
            base,
            read: X::default(),
            write: X::default(),
        }
    }
    fn base(&self) -> &SharedRb<S> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut SharedRb<S> {
        &mut self.base
    }
}

impl<S: Storage, X: Semaphore> Observer for BlockingRb<S, X> {
    delegate_observer!(SharedRb<S>, Self::base);
}
impl<S: Storage, X: Semaphore> Producer for BlockingRb<S, X> {
    unsafe fn set_write_index(&self, value: usize) {
        self.write.notify(|| self.base.set_write_index(value));
    }
}
impl<S: Storage, X: Semaphore> Consumer for BlockingRb<S, X> {
    unsafe fn set_read_index(&self, value: usize) {
        self.read.notify(|| self.base.set_read_index(value));
    }
}
impl<S: Storage, X: Semaphore> RingBuffer for BlockingRb<S, X> {
    delegate_ring_buffer!(Self::base, Self::base_mut);
}

impl<S: Storage, X: Semaphore> BlockingProducer for BlockingRb<S, X> {
    type Instant = X::Instant;
    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.read.wait(|| self.vacant_len() >= count, timeout)
    }
}
impl<S: Storage, X: Semaphore> BlockingConsumer for BlockingRb<S, X> {
    type Instant = X::Instant;
    fn wait_occupied(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.write.wait(|| self.occupied_len() >= count, timeout)
    }
}

impl<S: Storage, X: Semaphore> AsRef<BlockingRb<S, X>> for BlockingRb<S, X> {
    fn as_ref(&self) -> &BlockingRb<S, X> {
        self
    }
}
unsafe impl<S: Storage, X: Semaphore> RbRef for BlockingRb<S, X> {
    type Target = BlockingRb<S, X>;
}

impl<S: Storage, X: Semaphore> SplitRef for BlockingRb<S, X> {
    type RefProd<'a> = BlockingProd<&'a Self> where Self: 'a;
    type RefCons<'a> = BlockingCons<&'a Self> where Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        unsafe { (BlockingProd::new(self), BlockingCons::new(self)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage, X: Semaphore> Split for BlockingRb<S, X> {
    type Prod = BlockingProd<Arc<Self>>;
    type Cons = BlockingCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let arc = Arc::new(self);
        unsafe { (BlockingProd::new(arc.clone()), BlockingCons::new(arc)) }
    }
}
