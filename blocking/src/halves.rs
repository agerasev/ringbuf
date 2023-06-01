use crate::traits::{BlockingConsumer, BlockingProducer};
use core::time::Duration;
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer,
    ref_::AsRb,
    traits::{Consumer, Observer, Producer},
};

pub struct BlockingProd<B: Producer + AsRb>
where
    B::Rb: BlockingProducer,
{
    base: B,
}
pub struct BlockingCons<B: Consumer + AsRb>
where
    B::Rb: BlockingConsumer,
{
    base: B,
}

impl<B: Producer + AsRb> BlockingProd<B>
where
    B::Rb: BlockingProducer,
{
    pub unsafe fn new(base: B) -> Self {
        Self { base }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}
impl<B: Consumer + AsRb> BlockingCons<B>
where
    B::Rb: BlockingConsumer,
{
    pub unsafe fn new(base: B) -> Self {
        Self { base }
    }
    fn base(&self) -> &B {
        &self.base
    }
    fn base_mut(&mut self) -> &mut B {
        &mut self.base
    }
}

impl<B: Producer + AsRb> Observer for BlockingProd<B>
where
    B::Rb: BlockingProducer,
{
    delegate_observer!(B, Self::base);
}
impl<B: Producer + AsRb> Producer for BlockingProd<B>
where
    B::Rb: BlockingProducer,
{
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

impl<B: Consumer + AsRb> Observer for BlockingCons<B>
where
    B::Rb: BlockingConsumer,
{
    delegate_observer!(B, Self::base);
}
impl<B: Consumer + AsRb> Consumer for BlockingCons<B>
where
    B::Rb: BlockingConsumer,
{
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

unsafe impl<B: Producer + AsRb> AsRb for BlockingProd<B>
where
    B::Rb: BlockingProducer,
{
    type Rb = B::Rb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_rb()
    }
}
unsafe impl<B: Consumer + AsRb> AsRb for BlockingCons<B>
where
    B::Rb: BlockingConsumer,
{
    type Rb = B::Rb;
    fn as_rb(&self) -> &Self::Rb {
        self.base.as_rb()
    }
}
