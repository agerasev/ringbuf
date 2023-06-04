use crate::{
    delegate_observer, impl_consumer_traits, impl_producer_traits,
    rb::{AsRb, RbRef},
    traits::{Consumer, Observe, Observer, Producer, RingBuffer},
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
}
impl<R: RbRef> Prod<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(rb: R) -> Self {
        Self { rb }
    }
    pub fn into_base_ref(self) -> R {
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
    pub fn into_base_ref(self) -> R {
        self.rb
    }
}

unsafe impl<R: RbRef> AsRb for Obs<R> {
    type Rb = R::Target;
    fn as_rb(&self) -> &Self::Rb {
        self.rb.deref()
    }
}
unsafe impl<R: RbRef> AsRb for Prod<R> {
    type Rb = R::Target;
    fn as_rb(&self) -> &Self::Rb {
        self.rb.deref()
    }
}
unsafe impl<R: RbRef> AsRb for Cons<R> {
    type Rb = R::Target;
    fn as_rb(&self) -> &Self::Rb {
        self.rb.deref()
    }
}

impl<R: RbRef> Observer for Obs<R> {
    delegate_observer!(R::Target, AsRb::as_rb);
}

impl<R: RbRef> Observer for Prod<R> {
    delegate_observer!(R::Target, AsRb::as_rb);
}

impl<R: RbRef> Observer for Cons<R> {
    delegate_observer!(R::Target, AsRb::as_rb);
}

impl<R: RbRef> Producer for Prod<R> {
    #[inline]
    unsafe fn set_write_index(&mut self, value: usize) {
        self.as_rb().unsafe_set_write_index(value)
    }
}

impl<R: RbRef> Consumer for Cons<R> {
    #[inline]
    unsafe fn set_read_index(&mut self, value: usize) {
        self.as_rb().unsafe_set_read_index(value)
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
