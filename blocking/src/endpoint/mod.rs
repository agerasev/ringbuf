mod cons;
mod prod;

use crate::rb::BlockingRbHandle;
use core::time::Duration;
use ringbuf::{
    Obs,
    endpoint::{Endpoint, cached::Cached},
    traits::Delegate,
};

pub struct BlockingEndpoint<R: BlockingRbHandle, const P: bool, const C: bool> {
    pub(crate) rb: R,
    pub(crate) base: Cached<R, P, C>,
    pub(crate) timeout: Option<Duration>,
    #[cfg(feature = "std")]
    deadline: Option<std::time::Instant>,
}

impl<R: BlockingRbHandle, const P: bool, const C: bool> BlockingEndpoint<R, P, C> {
    pub fn new(rb: R) -> Self {
        Self {
            rb: rb.clone(),
            base: Cached::new(rb),
            timeout: None,
            #[cfg(feature = "std")]
            deadline: None,
        }
    }

    pub fn from_cached(base: Cached<R, P, C>) -> Self {
        Self {
            rb: base.rb_handle().clone(),
            base,
            timeout: None,
            #[cfg(feature = "std")]
            deadline: None,
        }
    }
    pub fn close(&mut self) {
        self.base.close();
    }
    #[cfg(feature = "std")]
    pub fn set_deadline(&mut self, deadline: Option<std::time::Instant>) {
        self.deadline = deadline;
    }
    #[cfg(feature = "std")]
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.deadline
    }
    fn remaining_timeout(&self) -> Option<Duration> {
        #[cfg(feature = "std")]
        if let Some(deadline) = self.deadline {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            return Some(self.timeout.map_or(left, |timeout| timeout.min(left)));
        }
        self.timeout
    }
    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}
impl<R: BlockingRbHandle, const P: bool, const C: bool> Delegate for BlockingEndpoint<R, P, C> {
    type Base = Cached<R, P, C>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}
unsafe impl<R: BlockingRbHandle, const P: bool, const C: bool> Endpoint for BlockingEndpoint<R, P, C> {
    type Handle = R;
    fn rb_handle(&self) -> &Self::Handle {
        &self.rb
    }
    fn into_rb_handle(self) -> Self::Handle {
        self.base.into_rb_handle()
    }
}

impl<R: BlockingRbHandle, const P: bool, const C: bool> AsRef<Self> for BlockingEndpoint<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: BlockingRbHandle, const P: bool, const C: bool> AsMut<Self> for BlockingEndpoint<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

pub use ringbuf::error::{TransferError, WaitError};

pub use cons::*;
pub use prod::*;

impl<R: BlockingRbHandle> BlockingEndpoint<R, true, false> {
    pub fn defer(&mut self) -> ringbuf::endpoint::DeferredProd<&mut ringbuf::endpoint::cached::Cached<R, true, false>> {
        self.base.defer()
    }
    pub fn into_deferred(self) -> ringbuf::endpoint::DeferredProd<ringbuf::endpoint::cached::Cached<R, true, false>> {
        self.base.into_deferred()
    }
}

impl<R: BlockingRbHandle> BlockingEndpoint<R, false, true> {
    pub fn defer(&mut self) -> ringbuf::endpoint::DeferredCons<&mut ringbuf::endpoint::cached::Cached<R, false, true>> {
        self.base.defer()
    }
    pub fn into_deferred(self) -> ringbuf::endpoint::DeferredCons<ringbuf::endpoint::cached::Cached<R, false, true>> {
        self.base.into_deferred()
    }
}
