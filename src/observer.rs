use crate::raw::RawRb;

pub trait Observer {
    type Item: Sized;

    type Raw: RawRb<Item = Self::Item>;

    fn as_raw(&self) -> &Self::Raw;

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    #[inline]
    fn capacity(&self) -> usize {
        self.as_raw().capacity().get()
    }

    /// Checks if the ring buffer is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.as_raw().is_empty()
    }

    /// Checks if the ring buffer is full.
    #[inline]
    fn is_full(&self) -> bool {
        self.as_raw().is_full()
    }

    /// The number of items stored in the buffer.
    #[inline]
    fn len(&self) -> usize {
        self.as_raw().occupied_len()
    }

    /// The number of remaining free places in the buffer.
    #[inline]
    fn free_len(&self) -> usize {
        self.as_raw().vacant_len()
    }
}
