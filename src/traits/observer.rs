use super::{
    Delegate,
    utils::{add_mod, modulus, sub_mod},
};
use core::{mem::MaybeUninit, num::NonZeroUsize};

/// Ring buffer observer.
///
/// Can observe ring buffer state but cannot safely access its data.
pub trait Observer {
    type Item: Sized;

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    /// Must not exceed `usize::MAX / 2` so that the index modulus is representable.
    fn capacity(&self) -> NonZeroUsize;

    /// Index of the last item in the ring buffer.
    ///
    /// Index value is in range `0..(2 * capacity)`.
    fn read_index(&self) -> usize;
    /// Index of the next empty slot in the ring buffer.
    ///
    /// Index value is in range `0..(2 * capacity)`.
    fn write_index(&self) -> usize;

    /// Oldest slot still retained by the consumer.
    fn read_released_index(&self) -> usize {
        self.read_index()
    }
    /// First initialized slot still owned by the backing ring buffer.
    fn read_claimed_index(&self) -> usize {
        self.read_index()
    }

    /// Queued items plus consumer-owned slots not yet released.
    fn retained_len(&self) -> usize {
        sub_mod(self.write_index(), self.read_released_index(), modulus(self))
    }
    /// Initialized items owned by the backing ring buffer.
    fn queued_len(&self) -> usize {
        sub_mod(self.write_index(), self.read_claimed_index(), modulus(self))
    }

    /// The number of items visible to this endpoint (legacy spelling).
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of producer or consumer respectively.*
    fn occupied_len(&self) -> usize {
        let modulus = modulus(self);
        sub_mod(self.write_index(), self.read_index(), modulus)
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of consumer or producer respectively.*
    fn vacant_len(&self) -> usize {
        let modulus = modulus(self);
        sub_mod(
            add_mod(self.read_released_index(), self.capacity().get(), modulus),
            self.write_index(),
            modulus,
        )
    }

    /// Checks if the ring buffer is empty.
    ///
    /// *The result may become irrelevant at any time because of concurring producer activity.*
    #[inline]
    fn is_empty(&self) -> bool {
        self.read_index() == self.write_index()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring consumer activity.*
    #[inline]
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }
}

/// Trait used for delegating observer methods.
///
/// # Safety
/// Both base accessors must always return the same live object. Observer overrides must preserve its capacity and bounds.
pub unsafe trait DelegateObserver: Delegate
where
    Self::Base: Observer + crate::traits::RawObserver,
{
}

impl<D: DelegateObserver> Observer for D
where
    D::Base: Observer + crate::traits::RawObserver,
{
    type Item = <D::Base as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base().capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.base().read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.base().write_index()
    }

    fn read_released_index(&self) -> usize {
        self.base().read_released_index()
    }
    fn read_claimed_index(&self) -> usize {
        self.base().read_claimed_index()
    }

    #[inline]
    fn occupied_len(&self) -> usize {
        self.base().occupied_len()
    }

    #[inline]
    fn vacant_len(&self) -> usize {
        self.base().vacant_len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.base().is_empty()
    }

    #[inline]
    fn is_full(&self) -> bool {
        self.base().is_full()
    }
}

impl<D: DelegateObserver> crate::traits::Presence for D
where
    D::Base: crate::traits::Presence + crate::traits::RawObserver,
{
    #[inline]
    fn read_is_held(&self) -> bool {
        self.base().read_is_held()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.base().write_is_held()
    }
}

unsafe impl<D: DelegateObserver> crate::traits::RawObserver for D
where
    D::Base: Observer + crate::traits::RawObserver,
{
    #[inline]
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.base().unsafe_slices(start, end) }
    }
    #[inline]
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.base().unsafe_slices_mut(start, end) }
    }
}

#[allow(unused_imports)]
use crate::traits::{RawConsumer, RawObserver, RawProducer, RawRingBuffer};

#[allow(unused_imports)]
use crate::traits::Presence;
