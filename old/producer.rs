#[cfg(feature = "std")]
use crate::utils::slice_assume_init_mut;
#[cfg(feature = "std")]
use core::cmp;
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

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

#[cfg(feature = "std")]
impl<R: RbRef> Producer<u8, R>
where
    R::Rb: RbWrite<u8>,
{
    /// Reads at most `count` bytes from `Read` instance and appends them to the ring buffer.
    /// If `count` is `None` then as much as possible bytes will be read.
    ///
    /// Returns `Ok(n)` if `read` succeeded. `n` is number of bytes been read.
    /// `n == 0` means that either `read` returned zero or ring buffer is full.
    ///
    /// If `read` is failed then original error is returned. In this case it is guaranteed that no items was read from the reader.
    /// To achieve this we read only one contiguous slice at once. So this call may read less than `remaining` items in the buffer even if the reader is ready to provide more.
    pub fn read_from<P: Read>(
        &mut self,
        reader: &mut P,
        count: Option<usize>,
    ) -> io::Result<usize> {
        let (left, _) = unsafe { self.free_space_as_slices() };
        let count = cmp::min(count.unwrap_or(left.len()), left.len());
        let left_init = unsafe { slice_assume_init_mut(&mut left[..count]) };

        let read_count = reader.read(left_init)?;
        assert!(read_count <= count);
        unsafe { self.advance(read_count) };
        Ok(read_count)
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> Write for Producer<u8, R>
where
    R::Rb: RbWrite<u8>,
{
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let n = self.push_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<R: RbRef> core::fmt::Write for Producer<u8, R>
where
    R::Rb: RbWrite<u8>,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let n = self.push_slice(s.as_bytes());
        if n != s.len() {
            Err(core::fmt::Error::default())
        } else {
            Ok(())
        }
    }
}
