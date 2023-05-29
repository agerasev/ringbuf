use crate::{
    frozen::{FrozenCons, FrozenProd},
    traits::{Consumer, Observer, Producer},
};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Deref};

use super::macros::{impl_cons_traits, impl_prod_traits};

/// Producer wrapper of ring buffer.
pub struct CachedProd<R: Deref>
where
    R::Target: Producer,
{
    frozen: FrozenProd<R>,
}

/// Consumer wrapper of ring buffer.
pub struct CachedCons<R: Deref>
where
    R::Target: Consumer,
{
    frozen: FrozenCons<R>,
}

impl<R: Deref> CachedProd<R>
where
    R::Target: Producer,
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
    R::Target: Consumer,
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
    R::Target: Producer,
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
    R::Target: Consumer,
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
    R::Target: Producer,
{
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

impl<R: Deref> Consumer for CachedCons<R>
where
    R::Target: Consumer,
{
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

impl_prod_traits!(CachedProd);
impl_cons_traits!(CachedCons);

impl<R: Deref> CachedProd<R>
where
    R::Target: Producer,
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
    R::Target: Consumer,
{
    pub fn freeze(&mut self) -> &mut FrozenCons<R> {
        &mut self.frozen
    }
    pub fn into_frozen(self) -> FrozenCons<R> {
        self.frozen
    }
}
