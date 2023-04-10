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
}
