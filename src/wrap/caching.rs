//! Caching implementation.
//!
//! Fetches changes from the ring buffer only when there is no more slots to perform requested operation.
//! Changes to this endpoint's index are always published before an operation returns.

#[allow(deprecated)]
use super::frozen::Frozen;
use super::{
    direct::{Direct, Obs},
    traits::Wrap,
};
use crate::{
    rb::RbRef,
    traits::{
        Observer,
        consumer::{Consumer, impl_consumer_traits},
        producer::{Producer, impl_producer_traits},
    },
};
use core::{cell::Cell, mem::MaybeUninit, num::NonZeroUsize};

/// Caching wrapper of a ring buffer.
pub struct Caching<R: RbRef, const P: bool, const C: bool> {
    base: Direct<R, P, C>,
    read: Cell<usize>,
    write: Cell<usize>,
}

/// Caching producer implementation.
pub type CachingProd<R> = Caching<R, true, false>;
/// Caching consumer implementation.
pub type CachingCons<R> = Caching<R, false, true>;

impl<R: RbRef, const P: bool, const C: bool> Caching<R, P, C> {
    /// Create a new ring buffer cached wrapper.
    ///
    /// Panics if wrapper with matching rights already exists.
    pub fn new(rb: R) -> Self {
        Self::from_direct(Direct::new(rb))
    }

    pub(crate) fn from_direct(base: Direct<R, P, C>) -> Self {
        Self {
            read: Cell::new(base.read_index()),
            write: Cell::new(base.write_index()),
            base,
        }
    }

    /// Get ring buffer observer.
    pub fn observe(&self) -> Obs<R> {
        self.base.observe()
    }

    /// Convert to the deprecated compatibility wrapper. Changes remain immediately visible.
    #[deprecated(note = "use the caching endpoint directly; freezing no longer delays publication")]
    #[allow(deprecated)]
    pub fn freeze(self) -> Frozen<R, P, C> {
        Frozen::from_caching(self)
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

impl<R: RbRef, const P: bool, const C: bool> Wrap for Caching<R, P, C> {
    type RbRef = R;

    fn rb_ref(&self) -> &R {
        self.base.rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.base.into_rb_ref()
    }
}

impl<R: RbRef, const P: bool, const C: bool> AsRef<Self> for Caching<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: RbRef, const P: bool, const C: bool> AsMut<Self> for Caching<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<R: RbRef, const P: bool, const C: bool> Observer for Caching<R, P, C> {
    type Item = <R::Rb as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        if P {
            self.fetch();
        }
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        if C {
            self.fetch();
        }
        self.write.get()
    }

    #[inline]
    fn read_is_held(&self) -> bool {
        self.base.read_is_held()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.base.write_is_held()
    }
}

unsafe impl<R: RbRef, const P: bool, const C: bool> crate::traits::RawObserver for Caching<R, P, C> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.base.unsafe_slices(start, end) }
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.base.unsafe_slices_mut(start, end) }
    }
}

impl<R: RbRef> Producer for CachingProd<R> {
    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
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

unsafe impl<R: RbRef> crate::traits::RawProducer for CachingProd<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
        unsafe { self.base.set_write_index(value) };
    }
}

impl<R: RbRef> Consumer for CachingCons<R> {
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

unsafe impl<R: RbRef> crate::traits::RawConsumer for CachingCons<R> {
    unsafe fn prepare_read(&mut self) {
        unsafe { self.base.prepare_read() };
        self.read.set(self.base.read_index());
    }

    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
        unsafe { self.base.set_read_index(value) };
    }
}

impl_producer_traits!(CachingProd<R: RbRef>);
impl_consumer_traits!(CachingCons<R: RbRef>);

#[allow(unused_imports)]
use crate::traits::{RawConsumer, RawObserver, RawProducer, RawRingBuffer};
