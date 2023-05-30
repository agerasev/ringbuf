use super::{
    based::{ConsRef, ProdRef},
    macros::*,
};
use crate::{
    //cached::FrozenCons,
    delegate_observer_methods,
    traits::{Consumer, Observer, Producer},
};
use core::{mem::MaybeUninit, ops::Deref};

/// Producer wrapper of ring buffer.
pub struct Prod<R: ProdRef> {
    ref_: R,
}

/// Consumer wrapper of ring buffer.
pub struct Cons<R: ConsRef> {
    ref_: R,
}

impl<R: ProdRef> Prod<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self { ref_ }
    }

    pub fn base_ref(&self) -> &R {
        &self.ref_
    }
    pub fn into_base_ref(self) -> R {
        self.ref_
    }
    #[inline]
    pub fn base(&self) -> &R::Base {
        self.ref_.base_deref()
    }
}

impl<R: ConsRef> Cons<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self { ref_ }
    }

    pub fn base_ref(&self) -> &R {
        &self.ref_
    }
    pub fn into_base_ref(self) -> R {
        self.ref_
    }
    #[inline]
    fn base(&self) -> &R::Base {
        self.ref_.base_deref()
    }
}

impl<R: ProdRef> Observer for Prod<R> {
    type Item = <R::Base as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: ConsRef> Observer for Cons<R> {
    type Item = <R::Base as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: ProdRef> Producer for Prod<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.base().set_write_index(value)
    }
}

impl<R: ConsRef> Consumer for Cons<R> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.base().set_read_index(value)
    }
}

impl_prod_traits!(Prod);
impl_cons_traits!(Cons);

impl_prod_freeze!(Prod);
impl_cons_freeze!(Cons);

impl<R: ProdRef> ProdRef for Prod<R> {
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
impl<R: ConsRef> ConsRef for Cons<R> {
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
