use core::ops::{Deref, DerefMut};

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
