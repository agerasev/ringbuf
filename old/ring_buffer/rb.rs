#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

/// An abstract reference to the ring buffer.
pub trait RbRef: Deref<Target = Self::Rb> {
    type Rb: ?Sized;
}
impl<B: ?Sized> RbRef for RbWrap<B> {
    type Rb = B;
}
impl<'a, B: ?Sized> RbRef for &'a B {
    type Rb = B;
}
#[cfg(feature = "alloc")]
impl<B: ?Sized> RbRef for Rc<B> {
    type Rb = B;
}
#[cfg(feature = "alloc")]
impl<B: ?Sized> RbRef for Arc<B> {
    type Rb = B;
}

/// Just a wrapper for a ring buffer.
///
/// Used to make an owning implementation of [`RbRef`].
#[repr(transparent)]
pub struct RbWrap<B: ?Sized>(pub B);
impl<B: ?Sized> Deref for RbWrap<B> {
    type Target = B;
    fn deref(&self) -> &B {
        &self.0
    }
}
impl<B: ?Sized> DerefMut for RbWrap<B> {
    fn deref_mut(&mut self) -> &mut B {
        &mut self.0
    }
}
