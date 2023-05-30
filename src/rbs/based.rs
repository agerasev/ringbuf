use crate::traits::RingBuffer;
#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

pub unsafe trait RbRef: Clone {
    type Target: RingBuffer;
    fn deref(&self) -> &Self::Target;
}
unsafe impl<'a, B: RingBuffer> RbRef for &'a B {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        self
    }
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer> RbRef for Rc<B> {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        self
    }
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer> RbRef for Arc<B> {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        self
    }
}

pub unsafe trait Based {
    type Rb: RingBuffer;
    type RbRef: RbRef<Target = Self::Rb>;
    fn rb(&self) -> &Self::Rb;
    fn rb_ref(&self) -> &Self::RbRef;
}
