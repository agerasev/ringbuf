use crate::traits::{Consumer, Producer, RingBuffer};
#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

pub unsafe trait RbRef: AsRef<Self::Target> {
    type Target: RingBuffer;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

unsafe impl<'a, B: RingBuffer> RbRef for &'a B
where
    B: RbRef<Target = B>,
{
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

pub trait ToRbRef {
    /// Ring buffer reference type.
    type RbRef: RbRef;

    /// Underlying ring buffer.
    fn rb(&self) -> &<Self::RbRef as RbRef>::Target {
        self.rb_ref().deref()
    }
    /// Underlying ring buffer reference.
    fn rb_ref(&self) -> &Self::RbRef;
    /// Destructure into underlying ring buffer reference.
    fn into_rb_ref(self) -> Self::RbRef;
}
