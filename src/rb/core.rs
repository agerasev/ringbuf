use super::{
    macros::rb_impl_init,
    utils::{assert_capacity, ranges},
};
#[cfg(feature = "alloc")]
use crate::traits::Split;
use crate::{
    endpoint::{CachedCons, CachedProd},
    storage::Storage,
    traits::{
        Observer, RingBuffer, SplitRef,
        consumer::{Consumer, impl_consumer_traits},
        producer::{Producer, impl_producer_traits},
    },
};
use crate::{indices::Indices, markers::Markers};
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

#[cfg(feature = "alloc")]
use {crate::alias::Arc, alloc::boxed::Box};

/// Ring buffer with independently selected storage, indices, and markers.
/// Storage is last so inline arrays can be coerced to dynamically sized slices.
pub struct Rb<S: Storage + ?Sized, I: Indices, M: Markers> {
    indices: I,
    markers: M,
    storage: S,
}

impl<S: Storage, I: Indices, M: Markers> Rb<S, I, M> {
    /// Constructs ring buffer from storage and indices.
    ///
    /// Panics if the storage is empty or its length exceeds `usize::MAX / 2`.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see implementation details).
    pub unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        assert_capacity(storage.len());
        Self {
            storage,
            indices: I::new(read, read, write),
            markers: M::default(),
        }
    }
    /// Destructures ring buffer into underlying storage and `read` and `write` indices.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize, usize) {
        let this = ManuallyDrop::new(self);
        (
            unsafe { ptr::read(&this.storage) },
            this.read_released_index(),
            this.read_claimed_index(),
            this.write_index(),
        )
    }
}

impl<S: Storage + ?Sized, I: Indices, M: Markers> Rb<S, I, M> {
    pub fn markers(&self) -> &M {
        &self.markers
    }
}

impl<S: Storage + ?Sized, I: Indices, M: Markers> Observer for Rb<S, I, M> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(self.storage.len()) }
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.indices.read_claimed()
    }
    fn read_released_index(&self) -> usize {
        self.indices.read_released()
    }
    fn read_claimed_index(&self) -> usize {
        self.indices.read_claimed()
    }

    #[inline]
    fn write_index(&self) -> usize {
        self.indices.write_published()
    }
}

impl<S: Storage + ?Sized, I: Indices, M: crate::markers::TrackedMarkers> crate::traits::Presence for Rb<S, I, M> {
    #[inline]
    fn read_is_held(&self) -> bool {
        self.markers.read_is_held()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.markers.write_is_held()
    }
}

unsafe impl<S: Storage + ?Sized, I: Indices, M: Markers> crate::traits::RawObserver for Rb<S, I, M> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<S::Item>], &[MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        unsafe { (self.storage.slice(first), self.storage.slice(second)) }
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        unsafe { (self.storage.slice_mut(first), self.storage.slice_mut(second)) }
    }
}

impl<S: Storage + ?Sized, I: Indices, M: Markers> Producer for Rb<S, I, M> {}

unsafe impl<S: Storage + ?Sized, I: Indices, M: Markers> crate::traits::RawProducer for Rb<S, I, M> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        unsafe { self.indices.set_write_published(value) };
    }
}

impl<S: Storage + ?Sized, I: Indices, M: Markers> Consumer for Rb<S, I, M> {}

unsafe impl<S: Storage + ?Sized, I: Indices, M: Markers> crate::traits::RawConsumer for Rb<S, I, M> {
    unsafe fn prepare_read(&mut self) {
        unsafe { self.indices.set_read_released(self.indices.read_claimed()) };
    }

    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        unsafe {
            self.indices.set_read_claimed(value);
            self.indices.set_read_released(value);
        }
    }
}

impl<S: Storage + ?Sized, I: Indices, M: Markers> RingBuffer for Rb<S, I, M> {}

unsafe impl<S: Storage + ?Sized, I: Indices, M: Markers> crate::traits::RawRingBuffer for Rb<S, I, M> {
    fn tracking_enabled(&self) -> bool {
        M::TRACKED
    }
    fn notify_read(&self) {
        self.markers.notify_read();
    }
    fn notify_write(&self) {
        self.markers.notify_write();
    }

    unsafe fn set_read_claimed(&self, value: usize) {
        unsafe { self.indices.set_read_claimed(value) };
    }
    unsafe fn set_read_released(&self, value: usize) {
        unsafe { self.indices.set_read_released(value) };
    }
    #[inline]
    unsafe fn hold_read(&self, flag: bool) -> bool {
        unsafe { self.markers.hold_read(flag) }
    }
    #[inline]
    unsafe fn hold_write(&self, flag: bool) -> bool {
        unsafe { self.markers.hold_write(flag) }
    }
}

impl<S: Storage + ?Sized, I: Indices, M: Markers> Drop for Rb<S, I, M> {
    fn drop(&mut self) {
        // Only this range belongs to the RB; abandoned claims may contain moved values.
        self.clear();
    }
}

#[cfg(feature = "alloc")]
impl<S: Storage, I: Indices, M: Markers> Split for Rb<S, I, M> {
    type Prod = CachedProd<Arc<Self>>;
    type Cons = CachedCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        unsafe { crate::endpoint::split_unchecked(Arc::new(self)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage + ?Sized, I: Indices, M: Markers> Split for Arc<Rb<S, I, M>> {
    type Prod = CachedProd<Self>;
    type Cons = CachedCons<Self>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        crate::endpoint::try_split(self).unwrap_or_else(|_| panic!("endpoint already held or tracking disabled"))
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage + ?Sized, I: Indices, M: Markers> Split for Box<Rb<S, I, M>> {
    type Prod = CachedProd<Arc<Rb<S, I, M>>>;
    type Cons = CachedCons<Arc<Rb<S, I, M>>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        unsafe { crate::endpoint::split_unchecked(Arc::<Rb<S, I, M>>::from(self)) }
    }
}
impl<S: Storage + ?Sized, I: Indices, M: Markers> SplitRef for Rb<S, I, M> {
    type RefProd<'a>
        = CachedProd<&'a Self>
    where
        Self: 'a;
    type RefCons<'a>
        = CachedCons<&'a Self>
    where
        Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        unsafe { crate::endpoint::split_unchecked(&*self) }
    }
}

rb_impl_init!(Rb, I: Indices, M: Markers);

impl_producer_traits!(Rb<S: Storage, I: Indices, M: Markers>);
impl_consumer_traits!(Rb<S: Storage, I: Indices, M: Markers>);

impl<S: Storage + ?Sized, I: Indices, M: Markers> AsRef<Self> for Rb<S, I, M> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<S: Storage + ?Sized, I: Indices, M: Markers> AsMut<Self> for Rb<S, I, M> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

#[allow(unused_imports)]
use crate::traits::{RawConsumer, RawObserver, RawProducer, RawRingBuffer};

#[allow(unused_imports)]
use crate::traits::Presence;
