mod frozen;
pub use frozen::*;

use super::{
    direct::Obs,
    frozen::{FrozenCons, FrozenProd},
    macros::*,
};
use crate::{
    rbs::ref_::RbRef,
    traits::{observer::Observe, Consumer, FrozenConsumer, FrozenProducer, Observer, Producer},
};
use core::{mem::MaybeUninit, num::NonZeroUsize};

/// Producer wrapper of ring buffer.
pub struct CachedProd<R: RbRef, const AUTO: bool = true> {
    frozen: FrozenProd<R>,
}

/// Consumer wrapper of ring buffer.
pub struct CachedCons<R: RbRef, const AUTO: bool = true> {
    frozen: FrozenCons<R>,
}

impl<R: RbRef, const AUTO: bool> CachedProd<R, AUTO> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            frozen: FrozenProd::new(ref_),
        }
    }
    pub fn into_rb_ref(self) -> R {
        self.frozen.into_rb_ref()
    }
}
impl<R: RbRef, const AUTO: bool> CachedCons<R, AUTO> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            frozen: FrozenCons::new(ref_),
        }
    }
    pub fn into_rb_ref(self) -> R {
        self.frozen.into_rb_ref()
    }
}

impl<R: RbRef, const AUTO: bool> Observer for CachedProd<R, AUTO> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.frozen.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        if AUTO {
            self.frozen.fetch();
        }
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

impl<R: RbRef, const AUTO: bool> Observer for CachedCons<R, AUTO> {
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
        if AUTO {
            self.frozen.fetch();
        }
        self.frozen.write_index()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.frozen.unsafe_slices(start, end)
    }
}

impl<R: RbRef, const AUTO: bool> Producer for CachedProd<R, AUTO> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.frozen.set_write_index(value);
        if AUTO {
            self.frozen.commit();
        }
    }

    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        if AUTO && self.frozen.is_full() {
            self.frozen.fetch();
        }
        let r = self.frozen.try_push(elem);
        if AUTO && r.is_ok() {
            self.frozen.commit();
        }
        r
    }
}

impl<R: RbRef, const AUTO: bool> Consumer for CachedCons<R, AUTO> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.frozen.set_read_index(value);
        if AUTO {
            self.frozen.commit();
        }
    }

    fn try_pop(&mut self) -> Option<<Self as Observer>::Item> {
        if AUTO && self.frozen.is_empty() {
            self.frozen.fetch();
        }
        let r = self.frozen.try_pop();
        if AUTO && r.is_some() {
            self.frozen.commit();
        }
        r
    }
}

// TODO: Handle AUTO
impl_prod_traits!(CachedProd);
impl_cons_traits!(CachedCons);

impl<R: RbRef> Observe for CachedProd<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.frozen.observe()
    }
}
impl<R: RbRef> Observe for CachedCons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.frozen.observe()
    }
}
