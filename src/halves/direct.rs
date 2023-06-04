use crate::{
    delegate_observer, impl_consumer_traits, impl_producer_traits,
    rb::RbRef,
    traits::{Consumer, Observe, Observer, Producer},
};

#[derive(Clone)]
pub struct Obs<R: RbRef> {
    rb: R,
}

/// Producer wrapper of ring buffer.
pub struct Prod<R: RbRef> {
    rb: R,
}

/// Consumer wrapper of ring buffer.
pub struct Cons<R: RbRef> {
    rb: R,
}

impl<R: RbRef> Obs<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub fn new(rb: R) -> Self {
        Self { rb }
    }
    pub fn rb(&self) -> &R::Target {
        self.rb.deref()
    }
    pub fn rb_ref(&self) -> &R {
        &self.rb
    }
    pub fn into_rb_ref(self) -> R {
        self.rb
    }
}
impl<R: RbRef> Prod<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(rb: R) -> Self {
        Self { rb }
    }
    pub fn rb(&self) -> &R::Target {
        self.rb.deref()
    }
    pub fn rb_ref(&self) -> &R {
        &self.rb
    }
    pub fn into_rb_ref(self) -> R {
        self.rb
    }
}
impl<R: RbRef> Cons<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(rb: R) -> Self {
        Self { rb }
    }
    pub fn rb(&self) -> &R::Target {
        self.rb.deref()
    }
    pub fn rb_ref(&self) -> &R {
        &self.rb
    }
    pub fn into_rb_ref(self) -> R {
        self.rb
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

impl_producer_traits!(Prod<R: RbRef>);
impl_consumer_traits!(Cons<R: RbRef>);

impl<R: RbRef> Observe for Obs<R> {
    type Obs = Self;
    fn observe(&self) -> Self::Obs {
        self.clone()
    }
}
impl<R: RbRef> Observe for Prod<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        Obs::new(self.rb.clone())
    }
}
impl<R: RbRef> Observe for Cons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        Obs::new(self.rb.clone())
    }
}
