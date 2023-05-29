use core::num::NonZeroUsize;

/// Modulus for pointers to item in ring buffer storage.
///
/// Equals to `2 * capacity`.
#[inline]
pub fn modulus(this: &impl Observer) -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(2 * this.capacity().get()) }
}

pub trait Observer: Sized {
    type Item: Sized;

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity(&self) -> NonZeroUsize;

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of producer or consumer respectively.*
    fn occupied_len(&self) -> usize;

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of consumer or producer respectively.*
    fn vacant_len(&self) -> usize;

    /// Checks if the ring buffer is empty.
    ///
    /// *The result may become irrelevant at any time because of concurring producer activity.*
    #[inline]
    fn is_empty(&self) -> bool {
        self.occupied_len() == 0
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring consumer activity.*
    #[inline]
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }
}

#[macro_export]
macro_rules! delegate_observer_methods {
    ($ref:expr) => {
        #[inline]
        fn capacity(&self) -> core::num::NonZeroUsize {
            $ref(self).capacity()
        }

        #[inline]
        fn occupied_len(&self) -> usize {
            $ref(self).occupied_len()
        }

        #[inline]
        fn vacant_len(&self) -> usize {
            $ref(self).vacant_len()
        }

        #[inline]
        fn is_empty(&self) -> bool {
            $ref(self).is_empty()
        }

        #[inline]
        fn is_full(&self) -> bool {
            $ref(self).is_full()
        }
    };
}
