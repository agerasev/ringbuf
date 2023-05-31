use super::{
    frozen::{FrozenCons, FrozenProd},
    macros::*,
};
use crate::{
    //cached::FrozenCons,
    delegate_observer,
    rbs::ref_::RbRef,
    traits::{observer::Observe, Consumer, Observer, Producer},
};

#[derive(Clone)]
pub struct Obs<R: RbRef> {
    ref_: R,
}

/// Producer wrapper of ring buffer.
pub struct Prod<R: RbRef> {
    ref_: R,
}

/// Consumer wrapper of ring buffer.
pub struct Cons<R: RbRef> {
    ref_: R,
}

impl<R: RbRef> Obs<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub fn new(ref_: R) -> Self {
        Self { ref_ }
    }

    #[inline]
    fn rb(&self) -> &R::Target {
        self.ref_.deref()
    }
}
impl<R: RbRef> Prod<R> {
    #[inline]
    fn rb(&self) -> &R::Target {
        self.ref_.deref()
    }
}
impl<R: RbRef> Cons<R> {
    #[inline]
    fn rb(&self) -> &R::Target {
        self.ref_.deref()
    }
}

impl<R: RbRef> Prod<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self { ref_ }
    }
    pub fn into_base_ref(self) -> R {
        self.ref_
    }
}

impl<R: RbRef> Cons<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(ref_: R) -> Self {
        Self { ref_ }
    }
    pub fn into_base_ref(self) -> R {
        self.ref_
    }
}

impl<R: RbRef> Observer for Obs<R> {
    delegate_observer!(R::Target, Self::rb);
}

impl<R: RbRef> Observer for Prod<R> {
    delegate_observer!(R::Target, Self::rb);
}

impl<R: RbRef> Observer for Cons<R> {
    delegate_observer!(R::Target, Self::rb);
}

impl<R: RbRef> Producer for Prod<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.rb().set_write_index(value)
    }
}

impl<R: RbRef> Consumer for Cons<R> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.rb().set_read_index(value)
    }
}

impl_prod_traits!(Prod);
impl_cons_traits!(Cons);

impl<R: RbRef> Prod<R> {
    pub fn freeze(&mut self) -> FrozenProd<&R::Target> {
        unsafe { FrozenProd::new(self.rb()) }
    }
    pub fn into_frozen(self) -> FrozenProd<R> {
        unsafe { FrozenProd::new(self.into_base_ref()) }
    }
}
impl<R: RbRef> Cons<R> {
    pub fn freeze(&mut self) -> FrozenCons<&R::Target> {
        unsafe { FrozenCons::new(self.rb()) }
    }
    pub fn into_frozen(self) -> FrozenCons<R> {
        unsafe { FrozenCons::new(self.into_base_ref()) }
    }
}

impl<R: RbRef> Observe for Obs<R> {
    type Obs = Self;
    fn observe(&self) -> Self::Obs {
        self.clone()
    }
}
impl<R: RbRef> Observe for Prod<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        Obs::new(self.ref_.clone())
    }
}
impl<R: RbRef> Observe for Cons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        Obs::new(self.ref_.clone())
    }
}
