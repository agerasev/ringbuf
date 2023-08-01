use crate::traits::{BlockingConsumer, BlockingProducer};
use core::time::Duration;
use ringbuf::{
    delegate_consumer, delegate_observer, delegate_producer, impl_consumer_traits, impl_producer_traits,
    rb::traits::{RbRef, ToRbRef},
    traits::{Consumer, Observe, Observer, Producer},
    CachingCons, CachingProd, Obs,
};

pub struct BlockingProd<R: RbRef> {
    base: CachingProd<R>,
}
pub struct BlockingCons<R: RbRef> {
    base: CachingCons<R>,
}

impl<R: RbRef> BlockingProd<R> {
    pub unsafe fn new(rb: R) -> Self {
        Self {
            base: CachingProd::new(rb),
        }
    }
    fn base(&self) -> &CachingProd<R> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut CachingProd<R> {
        &mut self.base
    }
}
impl<R: RbRef> ToRbRef for BlockingProd<R> {
    type RbRef = R;
    fn rb_ref(&self) -> &Self::RbRef {
        self.base.rb_ref()
    }
    fn into_rb_ref(self) -> Self::RbRef {
        self.base.into_rb_ref()
    }
}
impl<R: RbRef> BlockingCons<R> {
    pub unsafe fn new(rb: R) -> Self {
        Self {
            base: CachingCons::new(rb),
        }
    }
    fn base(&self) -> &CachingCons<R> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut CachingCons<R> {
        &mut self.base
    }
}
impl<R: RbRef> ToRbRef for BlockingCons<R> {
    type RbRef = R;
    fn rb_ref(&self) -> &Self::RbRef {
        self.base.rb_ref()
    }
    fn into_rb_ref(self) -> Self::RbRef {
        self.base.into_rb_ref()
    }
}

impl<R: RbRef> Observer for BlockingProd<R> {
    delegate_observer!(CachingProd<R>, Self::base);
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
    delegate_observer!(CachingCons<R>, Self::base);
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

impl<R: RbRef + Clone> Observe for BlockingProd<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.base.observe()
    }
}
impl<R: RbRef + Clone> Observe for BlockingCons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.base.observe()
    }
}
