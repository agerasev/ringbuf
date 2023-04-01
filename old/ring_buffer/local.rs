#[cfg(feature = "alloc")]
use alloc::rc::Rc;

impl<S: Storage> LocalRb<S> {
    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in [`Rc`]. If you don't want to use heap the see [`Self::split_ref`].
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Producer<T, Rc<Self>>, Consumer<T, Rc<Self>>)
    where
        Self: Sized,
    {
        let rc = Rc::new(self);
        unsafe { (Producer::new(rc.clone()), Consumer::new(rc)) }
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you also need to store the buffer somewhere.
    pub fn split_ref(&mut self) -> (Producer<T, &Self>, Consumer<T, &Self>)
    where
        Self: Sized,
    {
        unsafe { (Producer::new(self), Consumer::new(self)) }
    }
}
