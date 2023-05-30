use crate::traits::Observer;
#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

pub trait Based: Sized {
    type Base: Observer;
    fn base_deref(&self) -> &Self::Base;
}
impl<'a, O: Observer> Based for &'a O {
    type Base = O;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
#[cfg(feature = "alloc")]
impl<O: Observer> Based for Rc<O> {
    type Base = O;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
#[cfg(feature = "alloc")]
impl<O: Observer> Based for Arc<O> {
    type Base = O;
    fn base_deref(&self) -> &Self::Base {
        self
    }
}
