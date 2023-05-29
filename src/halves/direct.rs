use super::macros::*;
use crate::{
    //cached::FrozenCons,
    delegate_observer_methods,
    traits::{Consumer, Observer, Producer},
};
use core::{mem::MaybeUninit, ops::Deref};

/// Producer wrapper of ring buffer.
pub struct Prod<R: Deref>
where
    R::Target: Producer,
{
    base: R,
}

/// Consumer wrapper of ring buffer.
pub struct Cons<R: Deref>
where
    R::Target: Consumer,
{
    base: R,
}

impl<R: Deref> Prod<R>
where
    R::Target: Producer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(base: R) -> Self {
        Self { base }
    }
    pub fn base(&self) -> &R {
        &self.base
    }
    pub fn into_base(self) -> R {
        self.base
    }
}

impl<R: Deref> Cons<R>
where
    R::Target: Consumer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(base: R) -> Self {
        Self { base }
    }
    pub fn base(&self) -> &R {
        &self.base
    }
    pub fn into_base(self) -> R {
        self.base
    }
}

impl<R: Deref> Observer for Prod<R>
where
    R::Target: Producer,
{
    type Item = <R::Target as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Deref> Observer for Cons<R>
where
    R::Target: Consumer,
{
    type Item = <R::Target as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Deref> Producer for Prod<R>
where
    R::Target: Producer,
{
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.base.set_write_index(value)
    }
}

impl<R: Deref> Consumer for Cons<R>
where
    R::Target: Consumer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.base.set_read_index(value)
    }
}

impl_prod_traits!(Prod);
impl_cons_traits!(Cons);

impl_prod_freeze!(Prod);
impl_cons_freeze!(Cons);
