pub trait Observer {
    type Item: Sized;

    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    fn capacity(&self) -> usize;

    /// The number of items stored in the buffer.
    ///
    /// *Actual number may be greater or less than returned value due to concurring activity of producer or consumer respectively.*
    fn occupied_len(&self) -> usize;

    /// The number of remaining free places in the buffer.
    ///
    /// *Actual number may be less or greater than returned value due to concurring activity of producer or consumer respectively.*
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
