use super::{direct::Obs, frozen::Frozen};
use crate::{
    rb::traits::{RbRef, ToRbRef},
    traits::{Consumer, Observer, Producer},
};
use core::{fmt, mem::MaybeUninit, num::NonZeroUsize};
#[cfg(feature = "std")]
use std::io;

/// Caching wrapper of ring buffer.
pub struct Caching<R: RbRef, const P: bool, const C: bool> {
    frozen: Frozen<R, P, C>,
}

pub type CachingProd<R> = Caching<R, true, false>;
pub type CachingCons<R> = Caching<R, false, true>;

impl<R: RbRef, const P: bool, const C: bool> Caching<R, P, C> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(rb: R) -> Self {
        Self { frozen: Frozen::new(rb) }
    }

    pub fn observe(&self) -> Obs<R> {
        self.frozen.observe()
    }

    pub fn freeze(self) -> Frozen<R, P, C> {
        self.frozen
    }
}

impl<R: RbRef, const P: bool, const C: bool> ToRbRef for Caching<R, P, C> {
    type RbRef = R;

    fn rb_ref(&self) -> &R {
        self.frozen.rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.frozen.into_rb_ref()
    }
}

impl<R: RbRef, const P: bool, const C: bool> Observer for Caching<R, P, C> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.frozen.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        if P {
            self.frozen.fetch();
        }
        self.frozen.read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        if C {
            self.frozen.fetch();
        }
        self.frozen.write_index()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.frozen.unsafe_slices(start, end)
    }
}

impl<R: RbRef> Producer for CachingProd<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.frozen.set_write_index(value);
        self.frozen.commit();
    }

    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        if self.frozen.is_full() {
            self.frozen.fetch();
        }
        let r = self.frozen.try_push(elem);
        if r.is_ok() {
            self.frozen.commit();
        }
        r
    }
}

impl<R: RbRef> Consumer for CachingCons<R> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.frozen.set_read_index(value);
        self.frozen.commit();
    }

    fn try_pop(&mut self) -> Option<<Self as Observer>::Item> {
        if self.frozen.is_empty() {
            self.frozen.fetch();
        }
        let r = self.frozen.try_pop();
        if r.is_some() {
            self.frozen.commit();
        }
        r
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> io::Write for CachingProd<R>
where
    Self: Producer<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        <Self as Producer>::write(self, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<R: RbRef> fmt::Write for CachingProd<R>
where
    Self: Producer<Item = u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        <Self as Producer>::write_str(self, s)
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> io::Read for CachingCons<R>
where
    Self: Consumer<Item = u8>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        <Self as Consumer>::read(self, buf)
    }
}
