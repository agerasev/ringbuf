#[cfg(feature = "std")]
use std::io::{self, Read, Write};

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

#[cfg(feature = "std")]
impl<R: RbRef> Consumer<u8, R>
where
    R::Rb: RbRead<u8>,
{
    /// Removes at most first `count` bytes from the ring buffer and writes them into a [`Write`] instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    ///
    /// Returns `Ok(n)` if `write` succeeded. `n` is number of bytes been written.
    /// `n == 0` means that either `write` returned zero or ring buffer is empty.
    ///
    /// If `write` is failed then original error is returned. In this case it is guaranteed that no items was written to the writer.
    /// To achieve this we write only one contiguous slice at once. So this call may write less than `len` items even if the writer is ready to get more.
    pub fn write_into<P: Write>(
        &mut self,
        writer: &mut P,
        count: Option<usize>,
    ) -> io::Result<usize> {
        let (left, _) = unsafe { self.as_uninit_slices() };
        let count = cmp::min(count.unwrap_or(left.len()), left.len());
        let left_init = unsafe { slice_assume_init_ref(&left[..count]) };

        let write_count = writer.write(left_init)?;
        assert!(write_count <= count);
        unsafe { self.advance(write_count) };
        Ok(write_count)
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> Read for Consumer<u8, R>
where
    R::Rb: RbRead<u8>,
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.pop_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }
}
