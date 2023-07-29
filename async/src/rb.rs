use crate::{
    halves::{AsyncCons, AsyncProd},
    traits::{AsyncConsumer, AsyncObserver, AsyncProducer, AsyncRingBuffer},
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use core::{
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};
use futures::task::AtomicWaker;
#[cfg(feature = "alloc")]
use ringbuf::traits::Split;
use ringbuf::{
    delegate_observer, delegate_ring_buffer, impl_consumer_traits, impl_producer_traits,
    storage::Storage,
    traits::{Consumer, Observer, Producer, RingBuffer, SplitRef},
    SharedRb,
};

pub struct AsyncRb<S: Storage> {
    base: SharedRb<S>,
    read: AtomicWaker,
    write: AtomicWaker,
    closed: AtomicBool,
}

impl<S: Storage> AsyncRb<S> {
    pub fn from(base: SharedRb<S>) -> Self {
        Self {
            base,
            read: AtomicWaker::default(),
            write: AtomicWaker::default(),
            closed: AtomicBool::new(false),
        }
    }
    fn base(&self) -> &SharedRb<S> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut SharedRb<S> {
        &mut self.base
    }
}

impl<S: Storage> Unpin for AsyncRb<S> {}

impl<S: Storage> Observer for AsyncRb<S> {
    delegate_observer!(SharedRb<S>, Self::base);
}
impl<S: Storage> Producer for AsyncRb<S> {
    unsafe fn set_write_index(&self, value: usize) {
        self.base.set_write_index(value);
        self.write.wake();
    }
}
impl<S: Storage> Consumer for AsyncRb<S> {
    unsafe fn set_read_index(&self, value: usize) {
        self.base.set_read_index(value);
        self.read.wake();
    }
}
impl<S: Storage> RingBuffer for AsyncRb<S> {
    delegate_ring_buffer!(Self::base, Self::base_mut);
}

impl<S: Storage> AsyncObserver for AsyncRb<S> {
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }
    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}
impl<S: Storage> AsyncProducer for AsyncRb<S> {
    fn register_read_waker(&self, waker: &Waker) {
        self.read.register(waker);
    }
}
impl<S: Storage> AsyncConsumer for AsyncRb<S> {
    fn register_write_waker(&self, waker: &Waker) {
        self.write.register(waker);
    }
}
impl<S: Storage> AsyncRingBuffer for AsyncRb<S> {
    fn wake_consumer(&self) {
        self.write.wake()
    }
    fn wake_producer(&self) {
        self.read.wake()
    }
}

impl<S: Storage> SplitRef for AsyncRb<S> {
    type RefProd<'a> = AsyncProd<&'a Self> where Self:  'a;
    type RefCons<'a> = AsyncCons<&'a Self> where Self:  'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        unsafe { (AsyncProd::new(self), AsyncCons::new(self)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage> Split for AsyncRb<S> {
    type Prod = AsyncProd<Arc<Self>>;
    type Cons = AsyncCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let arc = Arc::new(self);
        unsafe { (AsyncProd::new(arc.clone()), AsyncCons::new(arc)) }
    }
}

impl_producer_traits!(AsyncRb<S: Storage>);
impl_consumer_traits!(AsyncRb<S: Storage>);
