use crate::traits::{BlockingConsumer, BlockingProducer};
use core::time::Duration;
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer, impl_consumer_traits, impl_producer_traits,
    rb::RbRef,
    traits::{Consumer, Observe, Observer, Producer},
    CachedCons, CachedProd, Obs,
};

pub struct BlockingProd<R: RbRef> {
    base: CachedProd<R>,
}
pub struct BlockingCons<R: RbRef> {
    base: CachedCons<R>,
}

impl<R: RbRef> BlockingProd<R> {
    pub unsafe fn new(rb: R) -> Self {
        Self { base: CachedProd::new(rb) }
    }
    fn base(&self) -> &CachedProd<R> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut CachedProd<R> {
        &mut self.base
    }
}
impl<R: RbRef> BlockingCons<R> {
    pub unsafe fn new(rb: R) -> Self {
        Self { base: CachedCons::new(rb) }
    }
    fn base(&self) -> &CachedCons<R> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut CachedCons<R> {
        &mut self.base
    }
}

impl<R: RbRef> Observer for BlockingProd<R> {
    delegate_observer!(CachedProd<R>, Self::base);
}
impl<R: RbRef> Producer for BlockingProd<R> {
    delegate_producer!(Self::base, Self::base_mut);
}
impl<R: RbRef> BlockingProducer for BlockingProd<R>
where
    R::Target: BlockingProducer,
{
    type Instant = <R::Target as BlockingProducer>::Instant;

    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base.rb().wait_vacant(count, timeout)
    }
}

impl<R: RbRef> Observer for BlockingCons<R> {
    delegate_observer!(CachedCons<R>, Self::base);
}
impl<R: RbRef> Consumer for BlockingCons<R> {
    delegate_consumer!(Self::base, Self::base_mut);
}
impl<R: RbRef> BlockingConsumer for BlockingCons<R>
where
    R::Target: BlockingConsumer,
{
    type Instant = <R::Target as BlockingConsumer>::Instant;

    fn wait_occupied(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base.rb().wait_occupied(count, timeout)
    }
}

impl_producer_traits!(BlockingProd<R: RbRef>);
impl_consumer_traits!(BlockingCons<R: RbRef>);

impl<R: RbRef> Observe for BlockingProd<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.base.observe()
    }
}
impl<R: RbRef> Observe for BlockingCons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.base.observe()
    }
}
