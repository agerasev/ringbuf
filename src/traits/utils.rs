use super::Observer;
use core::num::NonZeroUsize;

/// Modulus for pointers to item in ring buffer storage.
///
/// Equals to `2 * capacity`.
#[inline]
pub fn modulus(this: &impl Observer) -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(2 * this.capacity().get()) }
}
