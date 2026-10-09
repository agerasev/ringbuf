//! Direct implementation.
//!
//! All changes are synchronized with the ring buffer immediately.

#[allow(deprecated)]
use super::frozen::Frozen;
use super::{cached::Cached, traits::Endpoint};
use crate::{
    rb::RbHandle,
    traits::{
        Observer,
        consumer::{Consumer, impl_consumer_traits},
        producer::{Producer, impl_producer_traits},
    },
};
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

/// Direct wrapper of a ring buffer.
pub struct Direct<R: RbHandle, const P: bool, const C: bool> {
    rb: R,
}

/// Observer of a ring buffer.
pub type Obs<R> = Direct<R, false, false>;
/// Producer of a ring buffer.
pub type Prod<R> = Direct<R, true, false>;
/// Consumer of a ring buffer.
pub type Cons<R> = Direct<R, false, true>;

impl<R: RbHandle> Clone for Obs<R> {
    fn clone(&self) -> Self {
        Self { rb: self.rb.clone() }
    }
}

impl<R: RbHandle, const P: bool, const C: bool> Direct<R, P, C> {
    /// Create a new ring buffer direct wrapper.
    ///
    /// Panics if wrapper with matching rights already exists.
    pub fn new(rb: R) -> Self {
        Self::try_new(rb).unwrap_or_else(|_| panic!("endpoint is held or ownership tracking is disabled"))
    }

    /// Acquire endpoint rights without panicking; returns the handle on failure.
    pub fn try_new(rb: R) -> Result<Self, (super::AcquireError, R)> {
        if (P || C) && !rb.rb().tracking_enabled() {
            return Err((super::AcquireError::Untracked, rb));
        }
        if P && unsafe { rb.rb().hold_write(true) } {
            return Err((super::AcquireError::ProducerHeld, rb));
        }
        if C && unsafe { rb.rb().hold_read(true) } {
            if P {
                unsafe { rb.rb().hold_write(false) };
            }
            return Err((super::AcquireError::ConsumerHeld, rb));
        }
        Ok(Self { rb })
    }

    /// Acquire endpoint rights without checking their prior state.
    /// # Safety
    /// No other endpoint or data view may hold any of the requested rights.
    /// A forgotten endpoint's stale marker may be replaced after exclusive access.
    pub unsafe fn new_unchecked(rb: R) -> Self {
        if P {
            unsafe { rb.rb().hold_write(true) };
        }
        if C {
            unsafe { rb.rb().hold_read(true) };
        }
        Self { rb }
    }

    /// Get ring buffer observer.
    pub fn observe(&self) -> Obs<R> {
        Obs { rb: self.rb.clone() }
    }

    /// Compatibility spelling for conversion to a deferred endpoint.
    #[deprecated(note = "use into_deferred")]
    #[allow(deprecated)]
    pub fn freeze(self) -> Frozen<R, P, C> {
        let base = Cached::from_direct(self);
        unsafe { super::Deferred::from_endpoint(base) }
    }

    /// # Safety
    ///
    /// Must not be used after this call.
    unsafe fn close(&mut self) {
        if P {
            unsafe { self.rb().hold_write(false) };
        }
        if C {
            unsafe { self.rb().hold_read(false) };
        }
        if P {
            self.rb().notify_write();
        }
        if C {
            self.rb().notify_read();
        }
    }
}

unsafe impl<R: RbHandle, const P: bool, const C: bool> Endpoint for Direct<R, P, C> {
    type Handle = R;
    fn rb_handle(&self) -> &R {
        &self.rb
    }
    fn into_rb_handle(mut self) -> R {
        unsafe {
            self.close();
            let this = ManuallyDrop::new(self);
            ptr::read(&this.rb)
        }
    }
}

impl<R: RbHandle, const P: bool, const C: bool> AsRef<Self> for Direct<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: RbHandle, const P: bool, const C: bool> AsMut<Self> for Direct<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<R: RbHandle, const P: bool, const C: bool> Observer for Direct<R, P, C> {
    type Item = <R::Rb as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.rb().capacity()
    }
    #[inline]
    fn read_index(&self) -> usize {
        self.rb().read_index()
    }
    fn read_released_index(&self) -> usize {
        self.rb().read_released_index()
    }
    fn read_claimed_index(&self) -> usize {
        self.rb().read_claimed_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.rb().write_index()
    }
}

impl<R: RbHandle, const P: bool, const C: bool> crate::traits::Presence for Direct<R, P, C>
where
    R::Rb: crate::traits::Presence,
{
    #[inline]
    fn read_is_held(&self) -> bool {
        self.rb().read_is_held()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.rb().write_is_held()
    }
}

unsafe impl<R: RbHandle, const P: bool, const C: bool> crate::traits::RawObserver for Direct<R, P, C> {
    #[inline]
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.rb().unsafe_slices(start, end) }
    }
    #[inline]
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.rb().unsafe_slices_mut(start, end) }
    }
}

impl<R: RbHandle> Producer for Prod<R> {}

unsafe impl<R: RbHandle> crate::traits::RawProducer for Prod<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        unsafe { self.rb().set_write_index(value) }
    }
}

impl<R: RbHandle> Consumer for Cons<R> {}

unsafe impl<R: RbHandle> crate::traits::RawConsumer for Cons<R> {
    unsafe fn prepare_read(&mut self) {
        unsafe { self.rb().set_read_released(self.rb().read_claimed_index()) };
    }

    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        unsafe { self.rb().set_read_index(value) }
    }
}

impl<R: RbHandle, const P: bool, const C: bool> Drop for Direct<R, P, C> {
    fn drop(&mut self) {
        unsafe { self.close() };
    }
}

impl_producer_traits!(Prod<R: RbHandle>);
impl_consumer_traits!(Cons<R: RbHandle>);

#[allow(unused_imports)]
use crate::traits::{RawConsumer, RawObserver, RawProducer, RawRingBuffer};

#[allow(unused_imports)]
use crate::traits::Presence;

impl<R: RbHandle> Direct<R, true, false> {
    /// Defer publication and acquisition until explicitly synchronized.
    pub fn into_deferred(self) -> super::DeferredProd<Self> {
        unsafe { super::Deferred::from_endpoint(self) }
    }
    /// Temporarily defer this endpoint. Drop commits; forgetting may leak items.
    pub fn defer(&mut self) -> super::DeferredProd<&mut Self> {
        unsafe { super::Deferred::from_endpoint(self) }
    }
}

impl<R: RbHandle> Direct<R, false, true> {
    /// Defer publication and acquisition until explicitly synchronized.
    pub fn into_deferred(self) -> super::DeferredCons<Self> {
        unsafe { super::Deferred::from_endpoint(self) }
    }
    /// Temporarily defer this endpoint. Drop commits; forgetting may leak items.
    pub fn defer(&mut self) -> super::DeferredCons<&mut Self> {
        unsafe { super::Deferred::from_endpoint(self) }
    }
}
