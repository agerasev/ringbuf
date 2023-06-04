use crate::{
    halves::{AsyncCons, AsyncProd},
    traits::{AsyncConsumer, AsyncObserver, AsyncProducer, AsyncRingBuffer},
};
use core::{
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};
use futures::task::AtomicWaker;
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer, impl_consumer_traits, impl_producer_traits,
    rb::{
        traits::{GenSplit, GenSplitRef},
        AsRb,
    },
    traits::{Consumer, Observer, Producer, RingBuffer, Split, SplitRef},
};

#[derive(Default)]
pub struct AsyncRb<B: RingBuffer> {
    base: B,
    read: AtomicWaker,
    write: AtomicWaker,
    closed: AtomicBool,
}

impl<B: RingBuffer> AsyncRb<B> {
    pub fn from(base: B) -> Self {
        Self {
            base,
            read: AtomicWaker::default(),
            write: AtomicWaker::default(),
            closed: AtomicBool::new(false),
        }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}

impl<B: RingBuffer> Unpin for AsyncRb<B> {}

impl<B: RingBuffer> Observer for AsyncRb<B> {
    delegate_observer!(B, Self::base);
}
impl<B: RingBuffer> Producer for AsyncRb<B> {
    delegate_producer!(Self::base, Self::base_mut);
}
impl<B: RingBuffer> Consumer for AsyncRb<B> {
    delegate_consumer!(Self::base, Self::base_mut);
}
impl<B: RingBuffer> RingBuffer for AsyncRb<B> {
    unsafe fn unsafe_set_write_index(&self, value: usize) {
        self.base.unsafe_set_write_index(value);
        self.write.wake();
    }
    unsafe fn unsafe_set_read_index(&self, value: usize) {
        self.base.unsafe_set_read_index(value);
        self.read.wake();
    }
}

impl<B: RingBuffer> AsyncObserver for AsyncRb<B> {
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }
    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}
impl<B: RingBuffer> AsyncProducer for AsyncRb<B> {
    fn register_read_waker(&self, waker: &Waker) {
        self.read.register(waker);
    }
}
impl<B: RingBuffer> AsyncConsumer for AsyncRb<B> {
    fn register_write_waker(&self, waker: &Waker) {
        self.write.register(waker);
    }
}
impl<B: RingBuffer> AsyncRingBuffer for AsyncRb<B> {}

impl<'a, B: RingBuffer + GenSplitRef<'a, Self> + 'a> SplitRef<'a> for AsyncRb<B> {
    type RefProd = AsyncProd<B::GenRefProd>;
    type RefCons = AsyncCons<B::GenRefCons>;

    fn split_ref(&'a mut self) -> (Self::RefProd, Self::RefCons) {
        let (prod, cons) = B::gen_split_ref(self);
        (AsyncProd::new(prod), AsyncCons::new(cons))
    }
}
impl<B: RingBuffer + GenSplit<Self>> Split for AsyncRb<B> {
    type Prod = AsyncProd<B::GenProd>;
    type Cons = AsyncCons<B::GenCons>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let (prod, cons) = B::gen_split(self);
        (AsyncProd::new(prod), AsyncCons::new(cons))
    }
}

impl_producer_traits!(AsyncRb<B: RingBuffer>);
impl_consumer_traits!(AsyncRb<B: RingBuffer>);

pub trait AsAsyncRb {
    type AsyncRb: AsyncRingBuffer;
    fn as_async_rb(&self) -> &Self::AsyncRb;
}
impl<B: AsRb> AsAsyncRb for B
where
    B::Rb: AsyncRingBuffer,
{
    type AsyncRb = B::Rb;
    fn as_async_rb(&self) -> &Self::AsyncRb {
        self.as_rb()
    }
}
