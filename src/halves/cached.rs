use super::{
    direct::Obs,
    frozen::{FrozenCons, FrozenProd},
};
use crate::{
    impl_consumer_traits, impl_producer_traits,
    rb::{
        iter::PopIter,
        traits::{RbRef, ToRbRef},
    },
    traits::{Consumer, Observe, Observer, Producer},
};
use core::{mem::MaybeUninit, num::NonZeroUsize};

/// Caching producer of ring buffer.
pub struct CachingProd<R: RbRef> {
    frozen: FrozenProd<R>,
}

/// Caching consumer of ring buffer.
pub struct CachingCons<R: RbRef> {
    frozen: FrozenCons<R>,
}

impl<R: RbRef> CachingProd<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            frozen: FrozenProd::new(ref_),
        }
    }
}
impl<R: RbRef> ToRbRef for CachingProd<R> {
    type RbRef = R;

    fn rb_ref(&self) -> &R {
        self.frozen.rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.frozen.into_rb_ref()
    }
}
impl<R: RbRef> CachingCons<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self {
            frozen: FrozenCons::new(ref_),
        }
    }
}
impl<R: RbRef> ToRbRef for CachingCons<R> {
    type RbRef = R;

    fn rb_ref(&self) -> &R {
        self.frozen.rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.frozen.into_rb_ref()
    }
}

impl<R: RbRef> Observer for CachingProd<R> {
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

impl<R: RbRef> Observer for CachingCons<R> {
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

    type IntoIter = PopIter<R>;
    fn into_iter(self) -> Self::IntoIter {
        unsafe { PopIter::new(self.frozen.into_rb_ref()) }
    }

    type PopIter<'a> = PopIter<&'a R::Target> where R:'a, R::Target: 'a;
    fn pop_iter(&mut self) -> Self::PopIter<'_> {
        unsafe { PopIter::new(self.frozen.rb_ref().deref()) }
    }
}

impl_producer_traits!(CachingProd<R: RbRef>);
impl_consumer_traits!(CachingCons<R: RbRef>);

impl<R: RbRef + Clone> Observe for CachingProd<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.frozen.observe()
    }
}
impl<R: RbRef + Clone> Observe for CachingCons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        self.frozen.observe()
    }
}
