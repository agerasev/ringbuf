use core::num::NonZeroUsize;

/// Modulus for pointers to item in ring buffer storage.
///
/// Equals to `2 * capacity`.
#[inline]
pub(crate) fn modulus(this: &impl Observer) -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(2 * this.capacity().get()) }
}

pub trait Observer: Sized {
    type Item: Sized;

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity(&self) -> NonZeroUsize;

    /// Read end position.
    fn read_index(&self) -> usize;

    /// Write end position.
    fn write_index(&self) -> usize;

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of producer or consumer respectively.*
    fn occupied_len(&self) -> usize {
        let modulus = modulus(self);
        (modulus.get() + self.write_index() - self.read_index()) % modulus
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be less or greater than returned value due to concurring activity of producer or consumer respectively.*
    fn vacant_len(&self) -> usize {
        let modulus = modulus(self);
        (self.capacity().get() + self.read_index() - self.write_index()) % modulus
    }

    /// Checks if the ring buffer is empty.
    ///
    /// *The result may become irrelevant at any time because of concurring producer activity.*
    #[inline]
    fn is_empty(&self) -> bool {
        self.read_index() == self.write_index()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring consumer activity.*
    #[inline]
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }
}
