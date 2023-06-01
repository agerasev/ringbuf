use crate::traits::{AsyncConsumer, AsyncObserver, AsyncProducer};
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer,
    rb::AsRb,
    traits::{Consumer, Observer, Producer},
};

pub struct AsyncProd<B: Producer + AsRb>
where
    B::Rb: AsyncProducer,
{
    base: B,
}
pub struct AsyncCons<B: Consumer + AsRb>
where
    B::Rb: AsyncConsumer,
{
    base: B,
}

impl<B: Producer + AsRb> AsyncProd<B>
where
    B::Rb: AsyncProducer,
{
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
impl<B: Consumer + AsRb> AsyncCons<B>
where
    B::Rb: AsyncConsumer,
{
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

impl<B: Producer + AsRb> Observer for AsyncProd<B>
where
    B::Rb: AsyncProducer,
{
    delegate_observer!(B, Self::base);
}
impl<B: Producer + AsRb> Producer for AsyncProd<B>
where
    B::Rb: AsyncProducer,
{
    delegate_producer!(Self::base, Self::base_mut);
}
impl<B: Producer + AsRb> AsyncObserver for AsyncProd<B>
where
    B::Rb: AsyncProducer,
{
    fn is_closed(&self) -> bool {
        self.as_rb().is_closed()
    }
    fn close(&self) {
        self.as_rb().close()
    }
}
impl<B: Producer + AsRb> AsyncProducer for AsyncProd<B>
where
    B::Rb: AsyncProducer,
{
    fn register_read_waker(&self, waker: &core::task::Waker) {
        self.as_rb().register_read_waker(waker)
    }
}

impl<B: Consumer + AsRb> Observer for AsyncCons<B>
where
    B::Rb: AsyncConsumer,
{
    delegate_observer!(B, Self::base);
}
impl<B: Consumer + AsRb> Consumer for AsyncCons<B>
where
    B::Rb: AsyncConsumer,
{
    delegate_consumer!(Self::base, Self::base_mut);
}
impl<B: Consumer + AsRb> AsyncObserver for AsyncCons<B>
where
    B::Rb: AsyncConsumer,
{
    fn is_closed(&self) -> bool {
        self.base.as_rb().is_closed()
    }
    fn close(&self) {
        self.as_rb().close()
    }
}
impl<B: Consumer + AsRb> AsyncConsumer for AsyncCons<B>
where
    B::Rb: AsyncConsumer,
{
    fn register_write_waker(&self, waker: &core::task::Waker) {
        self.as_rb().register_write_waker(waker)
    }
}

impl<B: Producer + AsRb> Drop for AsyncProd<B>
where
    B::Rb: AsyncProducer,
{
    fn drop(&mut self) {
        self.close()
    }
}
impl<B: Consumer + AsRb> Drop for AsyncCons<B>
where
    B::Rb: AsyncConsumer,
{
    fn drop(&mut self) {
        self.close()
    }
}

unsafe impl<B: Producer + AsRb> AsRb for AsyncProd<B>
where
    B::Rb: AsyncProducer,
{
    type Rb = B::Rb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_rb()
    }
}
unsafe impl<B: Consumer + AsRb> AsRb for AsyncCons<B>
where
    B::Rb: AsyncConsumer,
{
    type Rb = B::Rb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_rb()
    }
}
