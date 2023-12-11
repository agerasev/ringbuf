mod cons;
mod prod;

use crate::rb::BlockingRbRef;
use core::time::Duration;
use ringbuf::{
    traits::Based,
    wrap::{caching::Caching, Wrap},
    Obs,
};

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
impl<R: BlockingRbRef, const P: bool, const C: bool> Wrap for BlockingWrap<R, P, C> {
    type RbRef = R;
    fn rb_ref(&self) -> &Self::RbRef {
        &self.rb
    }
    fn into_rb_ref(self) -> Self::RbRef {
        self.base.into_rb_ref()
    }
}

impl<R: BlockingRbRef, const P: bool, const C: bool> AsRef<Self> for BlockingWrap<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: BlockingRbRef, const P: bool, const C: bool> AsMut<Self> for BlockingWrap<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WaitError {
    TimedOut,
    Closed,
}

pub use cons::*;
pub use prod::*;
