//! Cached implementation.
//!
//! Fetches changes from the ring buffer only when there is no more slots to perform requested operation.
//! Changes to this endpoint's index are always published before an operation returns.

#[allow(deprecated)]
use super::frozen::Frozen;
use super::{
    direct::{Direct, Obs},
    traits::Endpoint,
};
use crate::{
    rb::RbHandle,
    traits::{
        Observer,
        consumer::{Consumer, impl_consumer_traits},
        producer::{Producer, impl_producer_traits},
    },
};
use core::{cell::Cell, mem::MaybeUninit, num::NonZeroUsize};

/// Cached wrapper of a ring buffer.
pub struct Cached<R: RbHandle, const P: bool, const C: bool> {
    base: Direct<R, P, C>,
    read: Cell<usize>,
    refresh: Cell<bool>,
    write: Cell<usize>,
}

/// Cached producer implementation.
pub type CachedProd<R> = Cached<R, true, false>;
/// Cached consumer implementation.
pub type CachedCons<R> = Cached<R, false, true>;

impl<R: RbHandle, const P: bool, const C: bool> Cached<R, P, C> {
    /// Create a new ring buffer cached wrapper.
    ///
    /// Panics if wrapper with matching rights already exists.
    pub fn new(rb: R) -> Self {
        Self::from_direct(Direct::new(rb))
    }

    /// Acquire a cached endpoint, returning the handle on failure.
    pub fn try_new(rb: R) -> Result<Self, (super::AcquireError, R)> {
        Direct::try_new(rb).map(Self::from_direct)
    }

    pub fn from_direct(base: Direct<R, P, C>) -> Self {
        Self {
            read: Cell::new(base.read_index()),
            refresh: Cell::new(true),
            write: Cell::new(base.write_index()),
            base,
        }
    }

    /// Get ring buffer observer.
    pub fn observe(&self) -> Obs<R> {
        self.base.observe()
    }

    /// Compatibility spelling for conversion to a deferred endpoint.
    #[deprecated(note = "use into_deferred")]
    #[allow(deprecated)]
    pub fn freeze(self) -> Frozen<R, P, C> {
        self.refresh.set(true);
        unsafe { super::Deferred::from_endpoint(self) }
    }

    pub(crate) fn fetch(&self) {
        if P {
            self.read.set(self.base.read_released_index());
        }
        if C {
            self.write.set(self.base.write_index());
        }
    }
}

unsafe impl<R: RbHandle, const P: bool, const C: bool> Endpoint for Cached<R, P, C> {
    type Handle = R;

    fn rb_handle(&self) -> &R {
        self.base.rb_handle()
    }
    fn into_rb_handle(self) -> R {
        self.base.into_rb_handle()
    }
}

impl<R: RbHandle, const P: bool, const C: bool> AsRef<Self> for Cached<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: RbHandle, const P: bool, const C: bool> AsMut<Self> for Cached<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<R: RbHandle, const P: bool, const C: bool> Observer for Cached<R, P, C> {
    type Item = <R::Rb as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        if C && self.refresh.get() {
            self.read.set(self.base.read_index());
        }
        if P {
            self.fetch();
        }
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        if P && self.refresh.get() {
            self.write.set(self.base.write_index());
        }
        if C {
            self.fetch();
        }
        self.write.get()
    }
}

impl<R: RbHandle, const P: bool, const C: bool> crate::traits::Presence for Cached<R, P, C>
where
    R::Rb: crate::traits::Presence,
{
    #[inline]
    fn read_is_held(&self) -> bool {
        self.base.read_is_held()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.base.write_is_held()
    }
}

unsafe impl<R: RbHandle, const P: bool, const C: bool> crate::traits::RawObserver for Cached<R, P, C> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.base.unsafe_slices(start, end) }
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.base.unsafe_slices_mut(start, end) }
    }
}

impl<R: RbHandle> Producer for CachedProd<R> {
    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        unsafe { self.prepare_write() };
        let capacity = self.capacity().get();
        if self.write.get().abs_diff(self.read.get()) == capacity {
            self.fetch();
        }
        if self.write.get().abs_diff(self.read.get()) == capacity {
            return Err(elem);
        }
        let write = self.write.get();
        // The cached read index only underestimates the available space.
        unsafe {
            self.unsafe_slices_mut(write, write + 1).0.get_unchecked_mut(0).write(elem);
            self.advance_write_index(1);
        }
        Ok(())
    }
}

unsafe impl<R: RbHandle> crate::traits::RawProducer for CachedProd<R> {
    unsafe fn prepare_write(&mut self) {
        if self.refresh.replace(false) {
            self.read.set(self.base.read_released_index());
            self.write.set(self.base.write_index());
        }
    }

    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
        unsafe { self.base.set_write_index(value) };
    }
}

impl<R: RbHandle> Consumer for CachedCons<R> {
    fn try_pop(&mut self) -> Option<<Self as Observer>::Item> {
        unsafe { self.prepare_read() };
        if self.read.get() == self.write.get() {
            self.fetch();
        }
        if self.read.get() == self.write.get() {
            return None;
        }
        let read = self.read.get();
        // Move the item out before publishing the slot for reuse.
        let item = unsafe { self.unsafe_slices(read, read + 1).0.get_unchecked(0).assume_init_read() };
        unsafe { self.advance_read_index(1) };
        Some(item)
    }
}

unsafe impl<R: RbHandle> crate::traits::RawConsumer for CachedCons<R> {
    unsafe fn prepare_read(&mut self) {
        if self.refresh.replace(false) {
            unsafe { self.base.prepare_read() };
            self.read.set(self.base.read_index());
            self.write.set(self.base.write_index());
        }
    }

    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
        unsafe { self.base.set_read_index(value) };
    }
}

impl_producer_traits!(CachedProd<R: RbHandle>);
impl_consumer_traits!(CachedCons<R: RbHandle>);

#[allow(unused_imports)]
use crate::traits::{RawConsumer, RawObserver, RawProducer, RawRingBuffer};

#[allow(unused_imports)]
use crate::traits::Presence;

impl<R: RbHandle> Cached<R, true, false> {
    /// Defer publication and acquisition until explicitly synchronized.
    pub fn into_deferred(self) -> super::DeferredProd<Self> {
        self.refresh.set(true);
        unsafe { super::Deferred::from_endpoint(self) }
    }
    /// Temporarily defer this endpoint. Drop commits; forgetting may leak items.
    pub fn defer(&mut self) -> super::DeferredProd<&mut Self> {
        self.refresh.set(true);
        unsafe { super::Deferred::from_endpoint(self) }
    }
}

impl<R: RbHandle> Cached<R, false, true> {
    /// Defer publication and acquisition until explicitly synchronized.
    pub fn into_deferred(self) -> super::DeferredCons<Self> {
        self.refresh.set(true);
        unsafe { super::Deferred::from_endpoint(self) }
    }
    /// Temporarily defer this endpoint. Drop commits; forgetting may leak items.
    pub fn defer(&mut self) -> super::DeferredCons<&mut Self> {
        self.refresh.set(true);
        unsafe { super::Deferred::from_endpoint(self) }
    }
}
