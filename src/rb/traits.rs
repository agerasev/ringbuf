use crate::traits::RingBuffer;
#[cfg(feature = "alloc")]
use {crate::alias::Arc, alloc::rc::Rc};

/// Abstract pointer to the owning ring buffer.
///
/// # Safety
///
/// Every access through `rb`, including through cloned handles, must refer to
/// the same live backing buffer. The buffer must outlive every reference returned
/// from a handle. A handle does not itself grant producer or consumer rights.
/// The default `rb` implementation requires `as_ref` to preserve this identity.
pub unsafe trait RbRef: Clone + AsRef<Self::Rb> {
    /// Underlying ring buffer.
    type Rb: RingBuffer + ?Sized;
    /// Get ring buffer reference.
    fn rb(&self) -> &Self::Rb {
        self.as_ref()
    }
}

unsafe impl<B: RingBuffer + AsRef<B> + ?Sized> RbRef for &B {
    type Rb = B;
    fn rb(&self) -> &B {
        // AsRef is safe to implement and may select a different buffer on each
        // call. A borrowed handle must always use the object it points to.
        self
    }
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer + ?Sized> RbRef for Rc<B> {
    type Rb = B;
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer + ?Sized> RbRef for Arc<B> {
    type Rb = B;
}
