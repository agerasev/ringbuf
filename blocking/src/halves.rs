use crate::traits::{BlockingConsumer, BlockingProducer};
use core::time::Duration;
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer, impl_consumer_traits, impl_producer_traits,
    rb::AsRb,
    traits::{Consumer, Observe, Observer, Producer},
};

pub struct BlockingProd<B: Producer> {
    base: B,
}
pub struct BlockingCons<B: Consumer> {
    base: B,
}

impl<B: Producer> BlockingProd<B> {
    pub fn from(base: B) -> Self {
        Self { base }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}
impl<B: Consumer> BlockingCons<B> {
    pub fn from(base: B) -> Self {
        Self { base }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}

impl<B: Producer> Observer for BlockingProd<B> {
    delegate_observer!(B, Self::base);
}
impl<B: Producer> Producer for BlockingProd<B> {
    delegate_producer!(Self::base, Self::base_mut);
}
impl<B: Producer + AsRb> BlockingProducer for BlockingProd<B>
where
    B::Rb: BlockingProducer,
{
    type Instant = <B::Rb as BlockingProducer>::Instant;

    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base.as_rb().wait_vacant(count, timeout)
    }
}

impl<B: Consumer> Observer for BlockingCons<B> {
    delegate_observer!(B, Self::base);
}
impl<B: Consumer> Consumer for BlockingCons<B> {
    delegate_consumer!(Self::base, Self::base_mut);
}
impl<B: Consumer + AsRb> BlockingConsumer for BlockingCons<B>
where
    B::Rb: BlockingConsumer,
{
    type Instant = <B::Rb as BlockingConsumer>::Instant;

    fn wait_occupied(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base.as_rb().wait_occupied(count, timeout)
    }
}

unsafe impl<B: Producer + AsRb> AsRb for BlockingProd<B> {
    type Rb = B::Rb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_rb()
    }
}
unsafe impl<B: Consumer + AsRb> AsRb for BlockingCons<B> {
    type Rb = B::Rb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_rb()
    }
}

impl_producer_traits!(BlockingProd<B: Producer>);
impl_consumer_traits!(BlockingCons<B: Consumer>);

impl<R: Producer + Observe> Observe for BlockingProd<R> {
    type Obs = R::Obs;
    fn observe(&self) -> Self::Obs {
        self.base.observe()
    }
}
impl<R: Consumer + Observe> Observe for BlockingCons<R> {
    type Obs = R::Obs;
    fn observe(&self) -> Self::Obs {
        self.base.observe()
    }
}
