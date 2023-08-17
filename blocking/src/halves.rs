use crate::traits::{BlockingConsumer, BlockingProducer};
use core::time::Duration;
use ringbuf::{
    impl_consumer_traits, impl_producer_traits,
    rb::traits::{RbRef, ToRbRef},
    traits::{
        delegate::{self, Delegate, DelegateMut},
        Observe,
    },
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

impl<R: RbRef> Delegate for BlockingProd<R> {
    type Base = CachingProd<R>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
}
impl<R: RbRef> DelegateMut for BlockingProd<R> {
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
impl<R: RbRef> delegate::Observer for BlockingProd<R> {}
impl<R: RbRef> delegate::Producer for BlockingProd<R> {}
impl<R: RbRef> BlockingProducer for BlockingProd<R>
where
    R::Target: BlockingProducer,
{
    type Instant = <R::Target as BlockingProducer>::Instant;

    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base.rb().wait_vacant(count, timeout)
    }
}

impl<R: RbRef> Delegate for BlockingCons<R> {
    type Base = CachingCons<R>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
}
impl<R: RbRef> DelegateMut for BlockingCons<R> {
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
impl<R: RbRef> delegate::Observer for BlockingCons<R> {}
impl<R: RbRef> delegate::Consumer for BlockingCons<R> {}
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
