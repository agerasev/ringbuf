use super::{BlockingWrap, WaitError};
use crate::{rb::BlockingRbRef, sync::Semaphore};
use core::time::Duration;
#[cfg(feature = "std")]
use ringbuf::traits::Based;
use ringbuf::{
    traits::{observer::DelegateObserver, producer::DelegateProducer, Observer, Producer},
    wrap::Wrap,
};
#[cfg(feature = "std")]
use std::io;

pub type BlockingProd<R> = BlockingWrap<R, true, false>;

impl<R: BlockingRbRef> DelegateObserver for BlockingProd<R> {}
impl<R: BlockingRbRef> DelegateProducer for BlockingProd<R> {}

macro_rules! wait_iter {
    ($self:expr) => {
        $self.rb.rb().read.take_iter($self.timeout()).reset()
    };
}

impl<R: BlockingRbRef> BlockingProd<R> {
    pub fn is_closed(&self) -> bool {
        !self.read_is_held()
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn wait_vacant(&mut self, count: usize) -> Result<(), WaitError> {
        debug_assert!(count <= self.rb().capacity().get());
        for _ in wait_iter!(self) {
            if self.base.vacant_len() >= count {
                return Ok(());
            }
            if self.is_closed() {
                return Err(WaitError::Closed);
            }
        }
        Err(WaitError::TimedOut)
    }

    pub fn push(&mut self, mut item: <Self as Observer>::Item) -> Result<(), (WaitError, <Self as Observer>::Item)> {
        for _ in wait_iter!(self) {
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

    pub fn push_all_iter<I: Iterator<Item = <Self as Observer>::Item>>(&mut self, iter: I) -> usize {
        let mut iter = iter.peekable();
        if iter.peek().is_none() {
            return 0;
        }
        let mut count = 0;
        for _ in wait_iter!(self) {
            if self.is_closed() {
                break;
            }

            count += self.base.push_iter(&mut iter);

            if iter.peek().is_none() {
                break;
            }
        }
        count
    }
}
impl<R: BlockingRbRef> BlockingProd<R>
where
    <Self as Observer>::Item: Copy,
{
    pub fn push_exact(&mut self, mut slice: &[<Self as Observer>::Item]) -> usize {
        if slice.is_empty() {
            return 0;
        }

        let mut count = 0;
        for _ in wait_iter!(self) {
            if self.is_closed() {
                break;
            }

            let n = self.base.push_slice(slice);
            slice = &slice[n..];
            count += n;

            if slice.is_empty() {
                break;
            }
        }
        count
    }
}

#[cfg(feature = "std")]
impl<R: BlockingRbRef> io::Write for BlockingProd<R>
where
    <Self as Based>::Base: Producer<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
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
