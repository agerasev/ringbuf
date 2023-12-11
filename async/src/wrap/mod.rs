mod cons;
mod prod;

use crate::rb::AsyncRbRef;
use ringbuf::{
    traits::{observer::DelegateObserver, Based},
    wrap::{direct::Direct, Wrap},
    Obs,
};

pub struct AsyncWrap<R: AsyncRbRef, const P: bool, const C: bool> {
    base: Option<Direct<R, P, C>>,
}

pub type AsyncProd<R> = AsyncWrap<R, true, false>;
pub type AsyncCons<R> = AsyncWrap<R, false, true>;

impl<R: AsyncRbRef, const P: bool, const C: bool> AsyncWrap<R, P, C> {
    pub unsafe fn new(rb: R) -> Self {
        Self {
            base: Some(Direct::new(rb)),
        }
    }

    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}

impl<R: AsyncRbRef, const P: bool, const C: bool> Based for AsyncWrap<R, P, C> {
    type Base = Direct<R, P, C>;
    fn base(&self) -> &Self::Base {
        self.base.as_ref().unwrap()
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        self.base.as_mut().unwrap()
    }
}

impl<R: AsyncRbRef, const P: bool, const C: bool> Wrap for AsyncWrap<R, P, C> {
    type RbRef = R;
    fn rb_ref(&self) -> &R {
        self.base().rb_ref()
    }
    fn into_rb_ref(self) -> R {
        self.base.unwrap().into_rb_ref()
    }
}

impl<R: AsyncRbRef, const P: bool, const C: bool> Unpin for AsyncWrap<R, P, C> {}

impl<R: AsyncRbRef, const P: bool, const C: bool> DelegateObserver for AsyncWrap<R, P, C> {}

impl<R: AsyncRbRef, const P: bool, const C: bool> AsRef<Self> for AsyncWrap<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: AsyncRbRef, const P: bool, const C: bool> AsMut<Self> for AsyncWrap<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}
