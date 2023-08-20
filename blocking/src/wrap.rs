use crate::rb::BlockingRbRef;
use core::time::Duration;
use ringbuf::{rb::traits::ToRbRef, traits::Based, wrap::caching::Caching, Obs};

pub struct BlockingWrap<R: BlockingRbRef, const P: bool, const C: bool> {
    pub(crate) rb: R,
    pub(crate) base: Caching<R, P, C>,
    pub(crate) timeout: Option<Duration>,
}

impl<R: BlockingRbRef, const P: bool, const C: bool> BlockingWrap<R, P, C> {
    pub fn new(rb: R) -> Self {
        Self {
            rb: rb.clone(),
            base: Caching::new(rb),
            timeout: None,
        }
    }

    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}
impl<R: BlockingRbRef, const P: bool, const C: bool> Based for BlockingWrap<R, P, C> {
    type Base = Caching<R, P, C>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
impl<R: BlockingRbRef, const P: bool, const C: bool> ToRbRef for BlockingWrap<R, P, C> {
    type RbRef = R;
    fn rb_ref(&self) -> &Self::RbRef {
        &self.rb
    }
    fn into_rb_ref(self) -> Self::RbRef {
        self.base.into_rb_ref()
    }
}
