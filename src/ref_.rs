#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};
use core::{marker::PhantomData, ops::Deref};

pub trait Family {
    type Ref<T>: Deref<Target = T>;
}

enum Never {}
pub struct RefFamily<'a> {
    _ghost: PhantomData<&'a ()>,
    _never: Never,
}
impl<'a> Family for RefFamily<'a> {
    type Ref<T> = &'a T;
}

#[cfg(feature = "alloc")]
pub enum RcFamily {}
#[cfg(feature = "alloc")]
impl Family for RcFamily {
    type Ref<T> = Rc<T>;
}

#[cfg(feature = "alloc")]
pub enum ArcFamily {}
#[cfg(feature = "alloc")]
impl Family for ArcFamily {
    type Ref<T> = Arc<T>;
}
