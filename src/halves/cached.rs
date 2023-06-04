use super::{
    direct::Obs,
    frozen::{FrozenCons, FrozenProd},
};
use crate::{
    impl_consumer_traits, impl_producer_traits,
    rb::{AsRb, RbRef},
    traits::{Consumer, Observe, Observer, Producer},
};
use core::{mem::MaybeUninit, num::NonZeroUsize};

/// Producer wrapper of ring buffer.
pub struct CachedProd<R: RbRef> {
    frozen: FrozenProd<R>,
}

/// Consumer wrapper of ring buffer.
pub struct CachedCons<R: RbRef> {
    frozen: FrozenCons<R>,
}

impl<R: RbRef> CachedProd<R> {
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
    pub fn fetch(&mut self) {
        self.frozen.fetch()
    }
}
impl<R: RbRef> CachedCons<R> {
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
    pub fn fetch(&mut self) {
        self.frozen.fetch()
    }
}

unsafe impl<R: RbRef> AsRb for CachedProd<R> {
    type Rb = R::Target;
    fn as_rb(&self) -> &Self::Rb {
        self.frozen.rb()
    }
}
unsafe impl<R: RbRef> AsRb for CachedCons<R> {
    type Rb = R::Target;
    fn as_rb(&self) -> &Self::Rb {
        self.frozen.rb()
    }
}

impl<R: RbRef> Observer for CachedProd<R> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.frozen.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.as_rb().read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.frozen.write_index()
    }

    fn is_empty(&self) -> bool {
        self.frozen.is_empty()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.frozen.unsafe_slices(start, end)
    }
}

impl<R: RbRef> Observer for CachedCons<R> {
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
        self.as_rb().write_index()
    }

    fn is_full(&self) -> bool {
        self.frozen.is_full()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.frozen.unsafe_slices(start, end)
    }
}

impl<R: RbRef> Producer for CachedProd<R> {
    #[inline]
    unsafe fn set_write_index(&mut self, value: usize) {
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

impl<R: RbRef> Consumer for CachedCons<R> {
    unsafe fn set_read_index(&mut self, value: usize) {
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

impl_producer_traits!(CachedProd<R: RbRef>);
impl_consumer_traits!(CachedCons<R: RbRef>);

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
