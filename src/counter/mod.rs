#[cfg(feature = "async")]
mod async_;
mod atomic;
mod local;

#[cfg(feature = "async")]
pub use async_::*;
pub use atomic::*;
pub use local::*;

use core::num::NonZeroUsize;

pub trait Counter: Sized {
    fn new(len: NonZeroUsize, head: usize, tail: usize) -> Self;

    fn len(&self) -> NonZeroUsize;
    fn head(&self) -> usize;
    fn tail(&self) -> usize;

    unsafe fn set_head(&self, value: usize);
    unsafe fn set_tail(&self, value: usize);

    #[inline]
    fn modulus(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(2 * self.len().get()) }
    }

    /// The number of elements stored in the buffer at the moment.
    fn occupied_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.tail() - self.head()) % modulus
    }

    /// The number of vacant places in the buffer at the moment.
    fn vacant_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.head() - self.tail() - self.len().get()) % modulus
    }

    /// Checks if the occupied range is empty.
    fn is_empty(&self) -> bool {
        self.head() == self.tail()
    }

    /// Checks if the vacant range is empty.
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }

    unsafe fn acquire_head(&self) -> LocalHeadCounter<'_, Self> {
        LocalHeadCounter::new(self)
    }

    unsafe fn acquire_tail(&self) -> LocalTailCounter<'_, Self> {
        LocalTailCounter::new(self)
    }
}
