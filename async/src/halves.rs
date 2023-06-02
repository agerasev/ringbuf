use crate::{
    rb::AsAsyncRb,
    traits::{AsyncConsumer, AsyncObserver, AsyncProducer},
};
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer, impl_consumer_traits, impl_producer_traits,
    rb::AsRb,
    traits::{Consumer, Observer, Producer},
};

pub struct AsyncProd<B: Producer + AsAsyncRb> {
    base: B,
}
pub struct AsyncCons<B: Consumer + AsAsyncRb> {
    base: B,
}

impl<B: Producer + AsAsyncRb> AsyncProd<B> {
    pub fn new(base: B) -> Self {
        Self { base }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}
impl<B: Consumer + AsAsyncRb> AsyncCons<B> {
    pub fn new(base: B) -> Self {
        Self { base }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}

impl<B: Producer + AsAsyncRb> Unpin for AsyncProd<B> {}
impl<B: Consumer + AsAsyncRb> Unpin for AsyncCons<B> {}

impl<B: Producer + AsAsyncRb> Observer for AsyncProd<B> {
    delegate_observer!(B, Self::base);
}
impl<B: Producer + AsAsyncRb> Producer for AsyncProd<B> {
    delegate_producer!(Self::base, Self::base_mut);
}
impl<B: Producer + AsAsyncRb> AsyncObserver for AsyncProd<B> {
    fn is_closed(&self) -> bool {
        self.as_async_rb().is_closed()
    }
    fn close(&self) {
        self.as_async_rb().close()
    }
}
impl<B: Producer + AsAsyncRb> AsyncProducer for AsyncProd<B> {
    fn register_read_waker(&self, waker: &core::task::Waker) {
        self.as_async_rb().register_read_waker(waker)
    }
}

impl<B: Consumer + AsAsyncRb> Observer for AsyncCons<B> {
    delegate_observer!(B, Self::base);
}
impl<B: Consumer + AsAsyncRb> Consumer for AsyncCons<B> {
    delegate_consumer!(Self::base, Self::base_mut);
}
impl<B: Consumer + AsAsyncRb> AsyncObserver for AsyncCons<B> {
    fn is_closed(&self) -> bool {
        self.base.as_async_rb().is_closed()
    }
    fn close(&self) {
        self.as_async_rb().close()
    }
}
impl<B: Consumer + AsAsyncRb> AsyncConsumer for AsyncCons<B> {
    fn register_write_waker(&self, waker: &core::task::Waker) {
        self.as_async_rb().register_write_waker(waker)
    }
}

impl<B: Producer + AsAsyncRb> Drop for AsyncProd<B> {
    fn drop(&mut self) {
        self.close()
    }
}
impl<B: Consumer + AsAsyncRb> Drop for AsyncCons<B> {
    fn drop(&mut self) {
        self.close()
    }
}

unsafe impl<B: Producer + AsAsyncRb> AsRb for AsyncProd<B> {
    type Rb = B::AsyncRb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_async_rb()
    }
}
unsafe impl<B: Consumer + AsAsyncRb> AsRb for AsyncCons<B> {
    type Rb = B::AsyncRb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_async_rb()
    }
}

impl_producer_traits!(AsyncProd<B: Producer + AsAsyncRb>);
impl_consumer_traits!(AsyncCons<B: Consumer + AsAsyncRb>);
