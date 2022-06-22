use core::ops::{Deref, DerefMut};

/// Just a wrapper for a ring buffer.
///
/// Used to make an owning implementation of [`RbRef`](`crate::ring_buffer::RbRef`).
#[repr(transparent)]
pub struct RbWrap<B>(pub B);

impl<B> Deref for RbWrap<B> {
    type Target = B;
    fn deref(&self) -> &B {
        &self.0
    }
}

impl<B> DerefMut for RbWrap<B> {
    fn deref_mut(&mut self) -> &mut B {
        &mut self.0
    }
}
