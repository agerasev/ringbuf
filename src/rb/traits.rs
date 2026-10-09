use crate::traits::RingBuffer;
#[cfg(feature = "alloc")]
use {crate::alias::Arc, alloc::rc::Rc};

/// Abstract pointer to the owning ring buffer.
///
/// # Safety
///
/// All clones and repeated `rb`/`as_ref` calls must refer to the same live RB.
/// The handle must keep that RB and its storage alive; cloning and dropping a
/// handle must not mutate endpoint rights or item storage.
pub unsafe trait RbHandle: Clone + AsRef<Self::Rb> {
    /// Underlying ring buffer.
    type Rb: RingBuffer + ?Sized;
    /// Get ring buffer reference.
    fn rb(&self) -> &Self::Rb {
        self.as_ref()
    }
}

unsafe impl<B: RingBuffer + AsRef<B> + ?Sized> RbHandle for &B {
    type Rb = B;
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer + ?Sized> RbHandle for Rc<B> {
    type Rb = B;
}
#[cfg(feature = "alloc")]
unsafe impl<B: RingBuffer + ?Sized> RbHandle for Arc<B> {
    type Rb = B;
}

pub use RbHandle as RbRef;
