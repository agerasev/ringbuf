use crate::{
    frozen::{FrozenCons, FrozenProd},
    traits::{Consumer, Observer, Producer, RingBuffer},
};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Deref};

use super::macros::{impl_cons_traits, impl_prod_traits};

/// Producer wrapper of ring buffer.
pub struct CachedProd<R: Deref>
where
    R::Target: RingBuffer,
{
    frozen: FrozenProd<R>,
}

/// Consumer wrapper of ring buffer.
pub struct CachedCons<R: Deref>
where
    R::Target: RingBuffer,
{
    frozen: FrozenCons<R>,
}

impl<R: Deref> CachedProd<R>
where
    R::Target: RingBuffer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(base: R) -> Self {
        Self {
            frozen: FrozenProd::new(base),
        }
    }
    pub fn base(&self) -> &R {
        &self.frozen.base
    }
    //pub fn into_base(self) -> R {
    //    self.frozen.base
    //}
}

impl<R: Deref> CachedCons<R>
where
    R::Target: RingBuffer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(base: R) -> Self {
        Self {
            frozen: FrozenCons::new(base),
        }
    }
    pub fn base(&self) -> &R {
        &self.frozen.base
    }
    //pub fn into_base(self) -> R {
    //    self.frozen.base
    //}
}

impl<R: Deref> Observer for CachedProd<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.frozen.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.frozen.fetch();
        self.frozen.read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.frozen.write_index()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.frozen.unsafe_slices(start, end)
    }
}

impl<R: Deref> Observer for CachedCons<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.frozen.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.frozen.read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.frozen.fetch();
        self.frozen.write_index()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.frozen.unsafe_slices(start, end)
    }
}

impl<R: Deref> Producer for CachedProd<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.frozen.set_write_index(value);
        self.frozen.commit();
    }
}

impl<R: Deref> Consumer for CachedCons<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.frozen.set_read_index(value);
        self.frozen.commit();
    }
}

impl_prod_traits!(CachedProd);
impl_cons_traits!(CachedCons);

impl<R: Deref> CachedProd<R>
where
    R::Target: RingBuffer,
{
    pub fn freeze(&mut self) -> &mut FrozenProd<R> {
        &mut self.frozen
    }
    pub fn into_frozen(self) -> FrozenProd<R> {
        self.frozen
    }
}

impl<R: Deref> CachedCons<R>
where
    R::Target: RingBuffer,
{
    pub fn freeze(&mut self) -> &mut FrozenCons<R> {
        &mut self.frozen
    }
    pub fn into_frozen(self) -> FrozenCons<R> {
        self.frozen
    }
}
