//! Deprecated compatibility wrappers for caching endpoints.
//!
//! All writes and removals are now published immediately. Use [`CachingProd`](super::CachingProd)
//! and [`CachingCons`](super::CachingCons) directly instead; these wrappers will be removed in the
//! next breaking release.
//!
//! # Migration
//!
//! Remove calls to `freeze`, `commit`, `fetch`, and `sync`. Endpoints publish their own changes
//! immediately and fetch the opposite endpoint's progress as needed. `discard` is now a no-op:
//! published items cannot be retracted. Stage items outside the ring buffer if rollback is needed,
//! and use bulk operations such as `push_slice` and `pop_slice` to batch index updates.

#![allow(deprecated)]

use super::{caching::Caching, direct::Obs, traits::Wrap};
#[cfg(feature = "std")]
use crate::traits::Consumer;
use crate::{
    rb::RbRef,
    traits::{
        Based,
        consumer::{DelegateConsumer, impl_consumer_traits},
        observer::DelegateObserver,
        producer::{DelegateProducer, Producer, impl_producer_traits},
    },
};

/// Compatibility wrapper with the same immediate publication behavior as [`Caching`].
///
/// Unlike earlier versions, this wrapper never defers publication until `commit` or drop.
#[deprecated(note = "use Caching directly; frozen endpoints now publish changes immediately and will be removed in the next breaking release")]
pub struct Frozen<R: RbRef, const P: bool, const C: bool> {
    inner: Caching<R, P, C>,
}

/// Deprecated producer wrapper. All inserted items are published immediately.
#[deprecated(note = "use CachingProd; writes are now published immediately")]
pub type FrozenProd<R> = Frozen<R, true, false>;

/// Deprecated consumer wrapper. All removals are published immediately.
#[deprecated(note = "use CachingCons; removals are now published immediately")]
pub type FrozenCons<R> = Frozen<R, false, true>;

impl<R: RbRef, const P: bool, const C: bool> Frozen<R, P, C> {
    /// Create a compatibility wrapper with immediate publication.
    ///
    /// Panics if an endpoint with matching rights already exists.
    pub fn new(rb: R) -> Self {
        Self::from_caching(Caching::new(rb))
    }

    pub(crate) fn from_caching(inner: Caching<R, P, C>) -> Self {
        Self { inner }
    }

    /// Get ring buffer observer.
    pub fn observe(&self) -> Obs<R> {
        self.inner.observe()
    }

    /// Does nothing: all changes have already been published.
    #[deprecated(note = "changes are published immediately; remove this call")]
    pub fn commit(&self) {}

    /// Refresh the cached opposite index. Operations also refresh it as needed.
    #[deprecated(note = "the caching endpoint fetches progress automatically; remove this call")]
    pub fn fetch(&self) {
        self.inner.fetch();
    }

    /// Refresh the cached opposite index. All local changes are already published.
    #[deprecated(note = "changes are published immediately and progress is fetched automatically; remove this call")]
    pub fn sync(&self) {
        self.inner.fetch();
    }
}

impl<R: RbRef> FrozenProd<R> {
    /// Does nothing: inserted items are already published and cannot be retracted.
    ///
    /// Stage items outside the ring buffer if they may need to be discarded.
    #[deprecated(
        note = "discard is now a no-op because writes are immediately published; stage items outside the ring buffer to support rollback"
    )]
    pub fn discard(&mut self) {}
}

impl<R: RbRef, const P: bool, const C: bool> Based for Frozen<R, P, C> {
    type Base = Caching<R, P, C>;

    fn base(&self) -> &Self::Base {
        &self.inner
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.inner
    }
}

impl<R: RbRef, const P: bool, const C: bool> Wrap for Frozen<R, P, C> {
    type RbRef = R;

    fn rb_ref(&self) -> &R {
        self.inner.rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.inner.into_rb_ref()
    }
}

impl<R: RbRef, const P: bool, const C: bool> AsRef<Self> for Frozen<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: RbRef, const P: bool, const C: bool> AsMut<Self> for Frozen<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<R: RbRef, const P: bool, const C: bool> DelegateObserver for Frozen<R, P, C> {}
impl<R: RbRef> DelegateProducer for FrozenProd<R> {}
impl<R: RbRef> DelegateConsumer for FrozenCons<R> {}

impl_producer_traits!(FrozenProd<R: RbRef>);
impl_consumer_traits!(FrozenCons<R: RbRef>);
