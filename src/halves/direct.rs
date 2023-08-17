use crate::{
    rb::traits::{RbRef, ToRbRef},
    traits::{
        consumer::Consumer,
        observer::{DelegateObserver, Observe},
        producer::Producer,
    },
};
use core::fmt;
#[cfg(feature = "std")]
use std::io;

/// Observer of ring buffer.
#[derive(Clone)]
pub struct Obs<R: RbRef> {
    rb: R,
}
/// Producer of ring buffer.
pub struct Prod<R: RbRef> {
    rb: R,
}
/// Consumer of ring buffer.
pub struct Cons<R: RbRef> {
    rb: R,
}

impl<R: RbRef> Obs<R> {
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
}
impl<R: RbRef> Cons<R> {
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(rb: R) -> Self {
        Self { rb }
    }
}
impl<R: RbRef> ToRbRef for Obs<R> {
    type RbRef = R;
    fn rb_ref(&self) -> &R {
        &self.rb
    }
    fn into_rb_ref(self) -> R {
        self.rb
    }
}
impl<R: RbRef> ToRbRef for Prod<R> {
    type RbRef = R;
    fn rb_ref(&self) -> &R {
        &self.rb
    }
    fn into_rb_ref(self) -> R {
        self.rb
    }
}
impl<R: RbRef> ToRbRef for Cons<R> {
    type RbRef = R;
    fn rb_ref(&self) -> &R {
        &self.rb
    }
    fn into_rb_ref(self) -> R {
        self.rb
    }
}

impl<R: RbRef> DelegateObserver for Obs<R> {
    type Delegate = R::Target;
    fn delegate(&self) -> &Self::Delegate {
        self.rb.deref()
    }
    fn delegate_mut(&mut self) -> &mut Self::Delegate {
        unreachable!()
    }
}

impl<R: RbRef> DelegateObserver for Prod<R> {
    type Delegate = R::Target;
    fn delegate(&self) -> &Self::Delegate {
        self.rb.deref()
    }
    fn delegate_mut(&mut self) -> &mut Self::Delegate {
        unreachable!()
    }
}

impl<R: RbRef> DelegateObserver for Cons<R> {
    type Delegate = R::Target;
    fn delegate(&self) -> &Self::Delegate {
        self.rb.deref()
    }
    fn delegate_mut(&mut self) -> &mut Self::Delegate {
        unreachable!()
    }
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

#[cfg(feature = "std")]
impl<R: RbRef> io::Write for Prod<R>
where
    Self: Producer<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        <Self as Producer>::write(self, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<R: RbRef> fmt::Write for Prod<R>
where
    Self: Producer<Item = u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        <Self as Producer>::write_str(self, s)
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> io::Read for Cons<R>
where
    Self: Consumer<Item = u8>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        <Self as Consumer>::read(self, buf)
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
        Obs::new(self.rb.clone())
    }
}
impl<R: RbRef> Observe for Cons<R> {
    type Obs = Obs<R>;
    fn observe(&self) -> Self::Obs {
        Obs::new(self.rb.clone())
    }
}
