use super::{macros::*, Based};
use crate::{
    direct::Obs,
    frozen::{FrozenCons, FrozenProd},
    traits::{observer::Observe, Consumer, Observer, Producer},
};
use core::{mem::MaybeUninit, num::NonZeroUsize};

/// Producer wrapper of ring buffer.
pub struct CachedProd<R: Based>
where
    R::Base: Producer,
{
    frozen: FrozenProd<R>,
}

/// Consumer wrapper of ring buffer.
pub struct CachedCons<R: Based>
where
    R::Base: Consumer,
{
    frozen: FrozenCons<R>,
}

impl<R: Based> CachedProd<R>
where
    R::Base: Producer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            frozen: FrozenProd::new(ref_),
        }
    }
    pub fn into_base_ref(self) -> R {
        self.frozen.release()
    }
}

impl<R: Based> CachedCons<R>
where
    R::Base: Consumer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            frozen: FrozenCons::new(ref_),
        }
    }
    pub fn into_base_ref(self) -> R {
        self.frozen.release()
    }
}

impl<R: Based> Observer for CachedProd<R>
where
    R::Base: Producer,
{
    type Item = <R::Base as Observer>::Item;

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

impl<R: Based> Observer for CachedCons<R>
where
    R::Base: Consumer,
{
    type Item = <R::Base as Observer>::Item;

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

impl<R: Based> Producer for CachedProd<R>
where
    R::Base: Producer,
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

impl<R: Based> Consumer for CachedCons<R>
where
    R::Base: Consumer,
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

impl_prod_freeze!(CachedProd);
impl_cons_freeze!(CachedCons);

impl<R: Based> Based for CachedProd<R>
where
    R::Base: Producer,
{
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
impl<R: Based> Based for CachedCons<R>
where
    R::Base: Consumer,
{
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}

impl<R: Based + Clone> Observe for CachedProd<R>
where
    R::Base: Producer,
{
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        unsafe { Obs::new(self.frozen.ref_.clone()) }
    }
}
impl<R: Based + Clone> Observe for CachedCons<R>
where
    R::Base: Consumer,
{
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        unsafe { Obs::new(self.frozen.ref_.clone()) }
    }
}
