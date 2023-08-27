use crate::traits::RingBuffer;
#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

pub unsafe trait RbRef: Clone + AsRef<Self::Rb> {
    type Rb: RingBuffer;
    fn rb(&self) -> &Self::Rb {
        self.as_ref()
    }
}

unsafe impl<'a, B: RingBuffer + AsRef<B>> RbRef for &'a B {
    type Rb = B;
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer> RbRef for Rc<B> {
    type Rb = B;
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer> RbRef for Arc<B> {
    type Rb = B;
}
