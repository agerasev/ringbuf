use super::utils::modulus;
use core::{mem::MaybeUninit, num::NonZeroUsize};

pub trait Observer: Sized {
    type Item: Sized;

    /// Capacity of the ring buffer.
    ///
    /// It is constant during the whole ring buffer lifetime.
    fn capacity(&self) -> NonZeroUsize;

    fn read_index(&self) -> usize;
    fn write_index(&self) -> usize;

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]);

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of producer or consumer respectively.*
    fn occupied_len(&self) -> usize {
        let modulus = modulus(self);
        (modulus.get() + self.write_index() - self.read_index()) % modulus
    }

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of consumer or producer respectively.*
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

#[macro_export]
macro_rules! delegate_observer {
    ($type:ty, $ref:expr) => {
        type Item = <$type as $crate::traits::Observer>::Item;

        #[inline]
        fn capacity(&self) -> core::num::NonZeroUsize {
            $ref(self).capacity()
        }

        #[inline]
        fn read_index(&self) -> usize {
            $ref(self).read_index()
        }
        #[inline]
        fn write_index(&self) -> usize {
            $ref(self).write_index()
        }

        #[inline]
        unsafe fn unsafe_slices(
            &self,
            start: usize,
            end: usize,
        ) -> (&mut [core::mem::MaybeUninit<Self::Item>], &mut [core::mem::MaybeUninit<Self::Item>]) {
            $ref(self).unsafe_slices(start, end)
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

pub trait Observe {
    type Obs: Observer;
    fn observe(&self) -> Self::Obs;
}
