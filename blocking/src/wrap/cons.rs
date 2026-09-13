use super::{BlockingWrap, WaitError};
use crate::{rb::BlockingRbRef, sync::Semaphore};
use core::time::Duration;
#[cfg(feature = "std")]
use ringbuf::traits::Based;
use ringbuf::{
    traits::{Consumer, Observer, consumer::DelegateConsumer, observer::DelegateObserver},
    wrap::Wrap,
};
#[cfg(feature = "std")]
use std::io;

pub type BlockingCons<R> = BlockingWrap<R, false, true>;

impl<R: BlockingRbRef> DelegateObserver for BlockingCons<R> {}
impl<R: BlockingRbRef> DelegateConsumer for BlockingCons<R> {}

macro_rules! wait_iter {
    ($self:expr) => {
        $self.rb.rb().write.take_iter($self.timeout()).reset()
    };
}

impl<R: BlockingRbRef> BlockingCons<R> {
    pub fn is_closed(&self) -> bool {
        !self.write_is_held()
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn wait_occupied(&mut self, count: usize) -> Result<(), WaitError> {
        debug_assert!(count <= self.rb().capacity().get());
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

impl<R: BlockingRbRef> BlockingCons<R>
where
    <Self as Observer>::Item: Copy,
{
    pub fn pop_exact(&mut self, mut slice: &mut [<Self as Observer>::Item]) -> usize {
        if slice.is_empty() {
            return 0;
        }
        let mut count = 0;
        for _ in wait_iter!(self) {
            let n = self.base.pop_slice(slice);
            slice = &mut slice[n..];
            count += n;

            if slice.is_empty() || (self.is_closed() && self.is_empty()) {
                break;
            }
        }
        count
    }

    #[cfg(feature = "alloc")]
    pub fn pop_until_end(&mut self, vec: &mut alloc::vec::Vec<<Self as Observer>::Item>) {
        if self.is_closed() && self.is_empty() {
            return;
        }
        for _ in wait_iter!(self) {
            loop {
                if vec.len() == vec.capacity() {
                    vec.reserve(vec.capacity().max(16));
                }
                let n = self.base.pop_slice_uninit(vec.spare_capacity_mut());
                if n == 0 {
                    break;
                }
                unsafe { vec.set_len(vec.len() + n) };
            }
            if self.is_closed() && self.is_empty() {
                break;
            }
        }
    }
}

#[cfg(feature = "std")]
impl<R: BlockingRbRef> io::Read for BlockingCons<R>
where
    <Self as Based>::Base: Consumer<Item = u8>,
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

pub struct PopAllIter<'a, R: BlockingRbRef> {
    owner: &'a mut BlockingCons<R>,
}

impl<R: BlockingRbRef> Iterator for PopAllIter<'_, R> {
    type Item = <R::Rb as Observer>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.owner.pop().ok()
    }
}
