/// Consumer part of ring buffer.
///
/// # Mode
///
/// It can operate in immediate (by default) or postponed mode.
/// Mode could be switched using [`Self::postponed`]/[`Self::into_postponed`] and [`Self::into_immediate`] methods.
///
/// + In immediate mode removed and inserted items are automatically synchronized with the other end.
/// + In postponed mode synchronization occurs only when [`Self::sync`] or [`Self::into_immediate`] is called or when `Self` is dropped.
///   The reason to use postponed mode is that multiple subsequent operations are performed faster due to less frequent cache synchronization.
pub struct Consumer<T, R: RbRef>
where
    R::Rb: RbRead<T>,
{
    target: R,
    _phantom: PhantomData<T>,
}

impl<T, R: RbRef> Consumer<T, R>
where
    R::Rb: RbRead<T>,
{
    /// Creates consumer from the ring buffer reference.
    ///
    /// # Safety
    ///
    /// There must be only one consumer containing the same ring buffer reference.
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

    /// Returns postponed consumer that borrows [`Self`].
    pub fn postponed(&mut self) -> Consumer<T, RbWrap<RbReadCache<T, &R::Rb>>> {
        unsafe { Consumer::new(RbWrap(RbReadCache::new(&self.target))) }
    }

    /// Transforms [`Self`] into postponed consumer.
    pub fn into_postponed(self) -> Consumer<T, RbWrap<RbReadCache<T, R>>> {
        unsafe { Consumer::new(RbWrap(RbReadCache::new(self.target))) }
    }
}

/// Postponed consumer.
pub type PostponedConsumer<T, R> = Consumer<T, RbWrap<RbReadCache<T, R>>>;

impl<T, R: RbRef> PostponedConsumer<T, R>
where
    R::Rb: RbRead<T>,
{
    /// Create new postponed consumer.
    ///
    /// # Safety
    ///
    /// There must be only one consumer containing the same ring buffer reference.
    pub unsafe fn new_postponed(target: R) -> Self {
        Consumer::new(RbWrap(RbReadCache::new(target)))
    }

    /// Synchronize changes with the ring buffer.
    ///
    /// Postponed consumer requires manual synchronization to make freed space visible for the producer.
    pub fn sync(&mut self) {
        self.target.0.sync();
    }

    /// Synchronize and transform back to immediate consumer.
    pub fn into_immediate(self) -> Consumer<T, R> {
        unsafe { Consumer::new(self.target.0.release()) }
    }
}
