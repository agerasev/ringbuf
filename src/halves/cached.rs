use super::{
    based::{ConsRef, ProdRef},
    macros::*,
};
use crate::{
    frozen::{FrozenCons, FrozenProd},
    traits::{Consumer, Observer, Producer},
};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Deref};

/// Producer wrapper of ring buffer.
pub struct CachedProd<R: ProdRef> {
    frozen: FrozenProd<R>,
}

/// Consumer wrapper of ring buffer.
pub struct CachedCons<R: ConsRef> {
    frozen: FrozenCons<R>,
}

impl<R: ProdRef> CachedProd<R> {
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

impl<R: ConsRef> CachedCons<R> {
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

impl<R: ProdRef> Observer for CachedProd<R> {
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

impl<R: ConsRef> Observer for CachedCons<R> {
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

impl<R: ProdRef> Producer for CachedProd<R> {
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

impl<R: ConsRef> Consumer for CachedCons<R> {
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

impl<R: ProdRef> ProdRef for CachedProd<R> {
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
impl<R: ConsRef> ConsRef for CachedCons<R> {
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
