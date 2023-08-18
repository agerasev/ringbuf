use crate::traits::{BlockingConsumer, BlockingProducer};
use core::time::Duration;
use ringbuf::{
    rb::traits::{RbRef, ToRbRef},
    traits::{consumer::DelegateConsumer, observer::DelegateObserver, producer::DelegateProducer, Based},
    wrap::caching::Caching,
    Obs,
};

pub struct BlockingWrap<R: RbRef, const P: bool, const C: bool> {
    base: Option<Caching<R, P, C>>,
}
pub type BlockingProd<R> = BlockingWrap<R, true, false>;
pub type BlockingCons<R> = BlockingWrap<R, false, true>;

impl<R: RbRef, const P: bool, const C: bool> BlockingWrap<R, P, C> {
    pub fn new(rb: R) -> Self {
        Self {
            base: Some(Caching::new(rb)),
        }
    }

    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}
impl<R: RbRef, const P: bool, const C: bool> Based for BlockingWrap<R, P, C> {
    type Base = Caching<R, P, C>;
    fn base(&self) -> &Self::Base {
        self.base.as_ref().unwrap()
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        self.base.as_mut().unwrap()
    }
}
impl<R: RbRef, const P: bool, const C: bool> ToRbRef for BlockingWrap<R, P, C> {
    type RbRef = R;
    fn rb_ref(&self) -> &Self::RbRef {
        self.base().rb_ref()
    }
    fn into_rb_ref(self) -> Self::RbRef {
        self.base.unwrap().into_rb_ref()
    }
}

impl<R: RbRef> DelegateObserver for BlockingProd<R> {}
impl<R: RbRef> DelegateProducer for BlockingProd<R> {}

impl<R: RbRef> BlockingProducer for BlockingProd<R>
where
    R::Target: BlockingProducer,
{
    type Instant = <R::Target as BlockingProducer>::Instant;

    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base().rb().wait_vacant(count, timeout)
    }
}

impl<R: RbRef> DelegateObserver for BlockingCons<R> {}
impl<R: RbRef> DelegateConsumer for BlockingCons<R> {}

impl<R: RbRef> BlockingConsumer for BlockingCons<R>
where
    R::Target: BlockingConsumer,
{
    type Instant = <R::Target as BlockingConsumer>::Instant;

    fn wait_occupied(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base().rb().wait_occupied(count, timeout)
    }
}
