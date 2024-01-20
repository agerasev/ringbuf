use crate::traits::RingBuffer;
#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

/// Abstract pointer to the owning ring buffer.
///
/// # Safety
///
/// Implementation must be fair (e.g. not replacing pointers between calls and so on).
pub unsafe trait RbRef: Clone + AsRef<Self::Rb> {
    /// Underlying ring buffer.
    type Rb: RingBuffer + ?Sized;
    /// Get ring buffer reference.
    fn rb(&self) -> &Self::Rb {
        self.as_ref()
    }
}

unsafe impl<'a, B: RingBuffer + AsRef<B> + ?Sized> RbRef for &'a B {
    type Rb = B;
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer + ?Sized> RbRef for Rc<B> {
    type Rb = B;
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer + ?Sized> RbRef for Arc<B> {
    type Rb = B;
}
