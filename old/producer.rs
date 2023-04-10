impl<T, R: RbRef> Producer<T, R>
where
    R::Rb: RbWrite<T>,
{
    /// Creates producer from the ring buffer reference.
    ///
    /// # Safety
    ///
    /// There must be only one producer containing the same ring buffer reference.
    pub unsafe fn new(target: R) -> Self {
        Self {
            target,
            _phantom: PhantomData,
        }
    }

    /// Returns reference to the underlying ring buffer.
    #[inline]
    pub fn rb(&self) -> &R::Rb {
        &self.target
    }

    /// Consumes `self` and returns underlying ring buffer reference.
    pub fn into_rb_ref(self) -> R {
        self.target
    }
}
