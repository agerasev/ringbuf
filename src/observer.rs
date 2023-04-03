use crate::raw::{RawRb, RawStorage};
use core::ops::Deref;

pub trait Observer {
    type Item: Sized;

    type Raw: RawRb<Item = Self::Item>;

    fn as_raw(&self) -> &Self::Raw;

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    fn capacity(&self) -> usize {
        self.as_raw().capacity().get()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// *The result may become irrelevant at any time because of concurring producer activity.*
    #[inline]
    fn is_empty(&self) -> bool {
        self.as_raw().is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring consumer activity.*
    #[inline]
    fn is_full(&self) -> bool {
        self.as_raw().is_full()
    }

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of producer or consumer respectively.*
    #[inline]
    fn occupied_len(&self) -> usize {
        self.as_raw().occupied_len()
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be less or greater than returned value due to concurring activity of producer or consumer respectively.*
    #[inline]
    fn vacant_len(&self) -> usize {
        self.as_raw().vacant_len()
    }
}

pub struct Wrap<R> {
    raw: R,
}

impl<R> Wrap<R>
where
    R: Sized,
{
    pub fn new(raw: R) -> Self {
        Self { raw }
    }
}

impl<R: Deref> Observer for Wrap<R>
where
    R::Target: RawRb + Sized,
{
    type Item = <R::Target as RawStorage>::Item;
    type Raw = R::Target;
    fn as_raw(&self) -> &Self::Raw {
        &self.raw
    }
}
