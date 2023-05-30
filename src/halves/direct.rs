use super::{macros::*, Based};
use crate::{
    //cached::FrozenCons,
    delegate_observer_methods,
    traits::{observer::Observe, Consumer, Observer, Producer},
};
use core::mem::MaybeUninit;

pub struct Obs<R: Based> {
    ref_: R,
}

/// Producer wrapper of ring buffer.
pub struct Prod<R: Based>
where
    R::Base: Producer,
{
    ref_: R,
}

/// Consumer wrapper of ring buffer.
pub struct Cons<R: Based>
where
    R::Base: Consumer,
{
    ref_: R,
}

impl<R: Based> Obs<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self { ref_ }
    }
    #[inline]
    pub fn base(&self) -> &R::Base {
        self.ref_.base_deref()
    }
}

impl<R: Based> Prod<R>
where
    R::Base: Producer,
{
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

impl<R: Based> Cons<R>
where
    R::Base: Consumer,
{
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

impl<R: Based> Observer for Obs<R> {
    type Item = <R::Base as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Based> Observer for Prod<R>
where
    R::Base: Producer,
{
    type Item = <R::Base as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Based> Observer for Cons<R>
where
    R::Base: Consumer,
{
    type Item = <R::Base as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Based> Producer for Prod<R>
where
    R::Base: Producer,
{
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.base().set_write_index(value)
    }
}

impl<R: Based> Consumer for Cons<R>
where
    R::Base: Consumer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.base().set_read_index(value)
    }
}

impl_prod_traits!(Prod);
impl_cons_traits!(Cons);

impl_prod_freeze!(Prod);
impl_cons_freeze!(Cons);

impl<R: Based> Based for Prod<R>
where
    R::Base: Producer,
{
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
impl<R: Based> Based for Cons<R>
where
    R::Base: Consumer,
{
    type Base = Self;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}

impl<R: Based + Clone> Observe for Prod<R>
where
    R::Base: Producer,
{
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        unsafe { Obs::new(self.ref_.clone()) }
    }
}
impl<R: Based + Clone> Observe for Cons<R>
where
    R::Base: Consumer,
{
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        unsafe { Obs::new(self.ref_.clone()) }
    }
}
