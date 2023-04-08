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

    /// Returns postponed producer that borrows [`Self`].
    pub fn postponed(&mut self) -> PostponedProducer<T, &R::Rb> {
        unsafe { Producer::new(RbWrap(RbWriteCache::new(&self.target))) }
    }

    /// Transforms [`Self`] into postponed producer.
    pub fn into_postponed(self) -> PostponedProducer<T, R> {
        unsafe { Producer::new(RbWrap(RbWriteCache::new(self.target))) }
    }
}

/// Postponed producer.
pub type PostponedProducer<T, R> = Producer<T, RbWrap<RbWriteCache<T, R>>>;

impl<T, R: RbRef> PostponedProducer<T, R>
where
    R::Rb: RbWrite<T>,
{
    /// Create new postponed producer.
    ///
    /// # Safety
    ///
    /// There must be only one producer containing the same ring buffer reference.
    pub unsafe fn new_postponed(target: R) -> Self {
        Producer::new(RbWrap(RbWriteCache::new(target)))
    }

    /// Synchronize changes with the ring buffer.
    ///
    /// Postponed producer requires manual synchronization to make pushed items visible for the consumer.
    pub fn sync(&mut self) {
        self.target.0.sync();
    }

    /// Don't publish and drop items inserted since last synchronization.
    pub fn discard(&mut self) {
        self.target.0.discard();
    }

    /// Synchronize and transform back to immediate producer.
    pub fn into_immediate(self) -> Producer<T, R> {
        unsafe { Producer::new(self.target.0.release()) }
    }
}
