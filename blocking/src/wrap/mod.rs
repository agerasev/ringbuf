mod cons;
mod prod;

use crate::rb::BlockingRbRef;
use core::time::Duration;
use ringbuf::{
    Obs,
    endpoint::{Endpoint, cached::Cached},
    traits::Delegate,
};

pub struct BlockingWrap<R: BlockingRbRef, const P: bool, const C: bool> {
    pub(crate) rb: R,
    pub(crate) base: Cached<R, P, C>,
    pub(crate) timeout: Option<Duration>,
}

impl<R: BlockingRbRef, const P: bool, const C: bool> BlockingWrap<R, P, C> {
    pub fn new(rb: R) -> Self {
        Self {
            rb: rb.clone(),
            base: Cached::new(rb),
            timeout: None,
        }
    }

    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}
impl<R: BlockingRbRef, const P: bool, const C: bool> Delegate for BlockingWrap<R, P, C> {
    type Base = Cached<R, P, C>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
unsafe impl<R: BlockingRbRef, const P: bool, const C: bool> Endpoint for BlockingWrap<R, P, C> {
    type Handle = R;
    fn rb_handle(&self) -> &Self::Handle {
        &self.rb
    }
    fn into_rb_handle(self) -> Self::Handle {
        self.base.into_rb_handle()
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
