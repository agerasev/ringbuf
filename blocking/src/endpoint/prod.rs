use super::{BlockingEndpoint, TransferError, WaitError};
use crate::{rb::BlockingRbHandle, sync::Semaphore};
use core::time::Duration;
#[cfg(feature = "std")]
use ringbuf::traits::Delegate;
use ringbuf::traits::{Observer, Producer, observer::DelegateObserver, producer::DelegateProducer};
#[cfg(feature = "std")]
use std::io;

pub type BlockingProd<R> = BlockingEndpoint<R, true, false>;

unsafe impl<R: BlockingRbHandle> DelegateObserver for BlockingProd<R> {}
unsafe impl<R: BlockingRbHandle> DelegateProducer for BlockingProd<R> {}

macro_rules! wait_iter {
    ($self:expr) => {
        $self.rb.rb().markers().read.take_iter($self.remaining_timeout()).reset()
    };
}

impl<R: BlockingRbHandle> BlockingProd<R> {
    pub fn is_closed(&self) -> bool {
        !self.base.is_active() || !self.read_is_held()
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn wait_vacant(&mut self, count: usize) -> Result<(), WaitError> {
        unsafe { self.prepare_write() };
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
            if self.is_closed() {
                return Err(WaitError::Closed);
            }
            if self.base.vacant_len() >= count {
                return Ok(());
            }
        }
        Err(WaitError::TimedOut)
    }

    pub fn push(&mut self, mut item: <Self as Observer>::Item) -> Result<(), (WaitError, <Self as Observer>::Item)> {
        for _ in wait_iter!(self) {
            if self.is_closed() {
                return Err((WaitError::Closed, item));
            }
            item = match self.base.try_push(item) {
                Ok(()) => return Ok(()),
                Err(item) => item,
            };
            if self.is_closed() {
                return Err((WaitError::Closed, item));
            }
        }
        Err((WaitError::TimedOut, item))
    }

    /// Stream from a caller-owned peekable iterator. Unsent values remain in it.
    pub fn push_all_iter<I: Iterator<Item = <Self as Observer>::Item>>(
        &mut self,
        iter: &mut core::iter::Peekable<I>,
    ) -> Result<usize, TransferError> {
        if iter.peek().is_none() {
            return Ok(0);
        }
        let mut count = 0;
        for _ in wait_iter!(self) {
            if self.is_closed() {
                return Err(TransferError {
                    completed: count,
                    reason: WaitError::Closed,
                });
            }
            count += self.base.push_iter(&mut *iter);
            if iter.peek().is_none() {
                return Ok(count);
            }
        }
        Err(TransferError {
            completed: count,
            reason: WaitError::TimedOut,
        })
    }
}
impl<R: BlockingRbHandle> BlockingProd<R>
where
    <Self as Observer>::Item: Copy,
{
    /// Stream the entire slice, retaining the completed prefix on error.
    /// The timeout covers the whole call, including all intermediate wakeups.
    pub fn push_all(&mut self, mut slice: &[<Self as Observer>::Item]) -> Result<usize, TransferError> {
        if slice.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
        for _ in wait_iter!(self) {
            if self.is_closed() {
                return Err(TransferError {
                    completed: count,
                    reason: WaitError::Closed,
                });
            }
            let n = self.base.push_slice(slice);
            slice = &slice[n..];
            count += n;
            if slice.is_empty() {
                return Ok(count);
            }
        }
        Err(TransferError {
            completed: count,
            reason: WaitError::TimedOut,
        })
    }
    /// Compatibility spelling for `push_all`.
    pub fn push_exact(&mut self, slice: &[<Self as Observer>::Item]) -> Result<usize, TransferError> {
        self.push_all(slice)
    }
}

#[cfg(feature = "std")]
impl<R: BlockingRbHandle> io::Write for BlockingProd<R>
where
    <Self as Delegate>::Base: Producer<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        for _ in wait_iter!(self) {
            if self.is_closed() {
                return Ok(0);
            }
            let n = self.base.push_slice(buf);
            if n > 0 {
                return Ok(n);
            }
        }
        Err(io::ErrorKind::TimedOut.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[allow(unused_imports)]
use ringbuf::traits::Presence;

use ringbuf::traits::RawProducer;
