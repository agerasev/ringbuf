use super::{delegate::Delegate, utils::modulus};
use core::{mem::MaybeUninit, num::NonZeroUsize};

pub trait Observer: Sized {
    type Item: Sized;

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity(&self) -> NonZeroUsize;

    fn read_index(&self) -> usize;
    fn write_index(&self) -> usize;

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]);

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of producer or consumer respectively.*
    fn occupied_len(&self) -> usize {
        let modulus = modulus(self);
        (modulus.get() + self.write_index() - self.read_index()) % modulus
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of consumer or producer respectively.*
    fn vacant_len(&self) -> usize {
        let modulus = modulus(self);
        (self.capacity().get() + self.read_index() - self.write_index()) % modulus
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

pub trait DelegateObserver: Delegate
where
    Self::Base: Observer,
{
}
impl<D: DelegateObserver> Observer for D
where
    D::Base: Observer,
{
    type Item = <D::Base as Observer>::Item;

    #[inline]
    fn capacity(&self) -> core::num::NonZeroUsize {
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

    #[inline]
    unsafe fn unsafe_slices(
        &self,
        start: usize,
        end: usize,
    ) -> (&mut [core::mem::MaybeUninit<Self::Item>], &mut [core::mem::MaybeUninit<Self::Item>]) {
        self.base().unsafe_slices(start, end)
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

pub trait Observe {
    type Obs: Observer;
    fn observe(&self) -> Self::Obs;
}
