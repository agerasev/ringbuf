use super::{BlockingEndpoint, TransferError, WaitError};
use crate::{rb::BlockingRbHandle, sync::Semaphore};
use core::time::Duration;
#[cfg(feature = "std")]
use ringbuf::traits::Delegate;
use ringbuf::traits::{Consumer, Observer, consumer::DelegateConsumer, observer::DelegateObserver};
#[cfg(feature = "std")]
use std::io;

pub type BlockingCons<R> = BlockingEndpoint<R, false, true>;

unsafe impl<R: BlockingRbHandle> DelegateObserver for BlockingCons<R> {}
unsafe impl<R: BlockingRbHandle> DelegateConsumer for BlockingCons<R> {}

macro_rules! wait_iter {
    ($self:expr) => {
        $self.rb.rb().markers().write.take_iter($self.remaining_timeout()).reset()
    };
}

impl<R: BlockingRbHandle> BlockingCons<R> {
    pub fn is_closed(&self) -> bool {
        !self.base.is_active() || !self.write_is_held()
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn wait_occupied(&mut self, count: usize) -> Result<(), WaitError> {
        unsafe { self.prepare_read() };
        if count > self.capacity().get() {
            return Err(WaitError::TooLarge {
                requested: count,
                capacity: self.capacity().get(),
            });
        }
        if count == 0 {
            return Ok(());
        }
        for _ in wait_iter!(self) {
            // Observe closure before checking the data so a final write
            // followed by close cannot be mistaken for an empty buffer.
            let closed = self.is_closed();
            if self.base.occupied_len() >= count {
                return Ok(());
            }
            if closed {
                return Err(WaitError::Closed);
            }
        }
        Err(WaitError::TimedOut)
    }

    pub fn pop(&mut self) -> Result<<Self as Observer>::Item, WaitError> {
        for _ in wait_iter!(self) {
            let closed = self.is_closed();
            if let Some(item) = self.base.try_pop() {
                return Ok(item);
            }
            if closed {
                return Err(WaitError::Closed);
            }
        }
        Err(WaitError::TimedOut)
    }

    pub fn pop_all_iter(&mut self) -> PopAllIter<'_, R> {
        PopAllIter { owner: self }
    }
}

impl<R: BlockingRbHandle> BlockingCons<R>
where
    <Self as Observer>::Item: Copy,
{
    /// Stream the entire slice, retaining the completed prefix on error.
    /// The timeout covers the whole call, including all intermediate wakeups.
    pub fn pop_all(&mut self, mut slice: &mut [<Self as Observer>::Item]) -> Result<usize, TransferError> {
        if slice.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
        for _ in wait_iter!(self) {
            let closed = self.is_closed();
            let n = self.base.pop_slice(slice);
            slice = &mut slice[n..];
            count += n;
            if slice.is_empty() {
                return Ok(count);
            }
            if closed {
                return Err(TransferError {
                    completed: count,
                    reason: WaitError::Closed,
                });
            }
        }
        Err(TransferError {
            completed: count,
            reason: WaitError::TimedOut,
        })
    }
    /// Compatibility spelling for `pop_all`.
    pub fn pop_exact(&mut self, slice: &mut [<Self as Observer>::Item]) -> Result<usize, TransferError> {
        self.pop_all(slice)
    }
}

#[cfg(feature = "std")]
impl<R: BlockingRbHandle> io::Read for BlockingCons<R>
where
    <Self as Delegate>::Base: Consumer<Item = u8>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        for _ in wait_iter!(self) {
            // If closure is observed, the following read sees the final data.
            let closed = self.is_closed();
            let n = self.base.pop_slice(buf);
            if n > 0 {
                return Ok(n);
            }
            if closed {
                return Ok(0);
            }
        }
        Err(io::ErrorKind::TimedOut.into())
    }
}

pub struct PopAllIter<'a, R: BlockingRbHandle> {
    owner: &'a mut BlockingCons<R>,
}

impl<R: BlockingRbHandle> Iterator for PopAllIter<'_, R> {
    type Item = <R::Rb as Observer>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.owner.pop().ok()
    }
}

#[allow(unused_imports)]
use ringbuf::traits::Presence;

use ringbuf::traits::RawConsumer;

#[cfg(feature = "alloc")]
impl<R: BlockingRbHandle> BlockingCons<R> {
    pub fn pop_until_end(&mut self, vec: &mut alloc::vec::Vec<<Self as Observer>::Item>) -> Result<usize, ringbuf::error::CollectError> {
        self.pop_into_vec(vec, usize::MAX)
    }
    /// Append at most `limit` items, reporting allocation failures and timeouts.
    pub fn pop_into_vec(
        &mut self,
        vec: &mut alloc::vec::Vec<<Self as Observer>::Item>,
        limit: usize,
    ) -> Result<usize, ringbuf::error::CollectError> {
        use ringbuf::error::CollectError;
        let mut count = 0;
        for _ in wait_iter!(self) {
            let closed = self.is_closed();
            loop {
                if count == limit {
                    return Ok(count);
                }
                if vec.len() == vec.capacity() {
                    vec.try_reserve((limit - count).min(vec.capacity().max(16)))
                        .map_err(|error| CollectError::Allocation { completed: count, error })?;
                }
                let take = (limit - count).min(vec.spare_capacity_mut().len());
                let n = self.base.pop_slice_uninit(&mut vec.spare_capacity_mut()[..take]);
                unsafe { vec.set_len(vec.len() + n) };
                count += n;
                if n == 0 {
                    break;
                }
            }
            if closed {
                return Ok(count);
            }
        }
        Err(CollectError::Wait(TransferError {
            completed: count,
            reason: WaitError::TimedOut,
        }))
    }
}
