mod cons;
mod prod;

use crate::rb::BlockingRbRef;
use core::time::Duration;
use ringbuf::{
    Obs,
    traits::Based,
    wrap::{Wrap, caching::Caching},
};

pub struct BlockingWrap<R: BlockingRbRef, const P: bool, const C: bool> {
    pub(crate) rb: R,
    pub(crate) base: Caching<R, P, C>,
    pub(crate) timeout: Option<Duration>,
}

impl<R: BlockingRbRef, const P: bool, const C: bool> BlockingWrap<R, P, C> {
    pub fn new(rb: R) -> Self
    where
        R: Clone,
    {
        Self {
            rb: rb.clone(),
            base: Caching::new(rb),
            timeout: None,
        }
    }

    pub fn observe(&self) -> Obs<R>
    where
        R: Clone,
    {
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

/// A blocking operation could not complete.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WaitError {
    /// The configured timeout expired.
    TimedOut,
    /// The opposite endpoint is closed.
    Closed,
    /// The requested threshold exceeds the buffer's capacity.
    TooLarge { requested: usize, capacity: usize },
}

pub use cons::*;
pub use prod::*;

// SAFETY: both base accessors return the same uniquely owned endpoint.
unsafe impl<R: BlockingRbRef, const P: bool, const C: bool> ringbuf::traits::DelegateRaw for BlockingWrap<R, P, C> where
    Self: ringbuf::traits::Observer
{
}

impl core::fmt::Display for WaitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TimedOut => f.write_str("wait timed out"),
            Self::Closed => f.write_str("opposite endpoint is closed"),
            Self::TooLarge { requested, capacity } => write!(f, "requested {requested} slots exceeds capacity {capacity}"),
        }
    }
}
impl core::error::Error for WaitError {}
