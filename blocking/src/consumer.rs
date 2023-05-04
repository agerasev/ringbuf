use crate::{index::BlockingIndex, utils::TimeoutIterator};
use ringbuf::{
    index::Index,
    storage::Storage,
    traits::{Consumer, Observer, RingBuffer},
    Cons, Rb,
};
use std::{ops::Deref, time::Duration};

pub trait BlockingConsumer: Consumer {
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool;

    fn pop_wait(&mut self, timeout: Option<Duration>) -> Option<Self::Item> {
        if self.wait_read(1, timeout) {
            Some(self.try_pop().unwrap())
        } else {
            None
        }
    }

    fn pop_iter_all(&mut self, timeout: Option<Duration>) -> PopAllIter<'_, Self> {
        PopAllIter {
            target: self,
            timeout: TimeoutIterator::new(timeout),
        }
    }

    fn pop_slice_all(&mut self, mut slice: &mut [Self::Item], timeout: Option<Duration>) -> usize
    where
        Self::Item: Copy,
    {
        let mut count = 0;
        for timeout in TimeoutIterator::new(timeout) {
            if slice.is_empty() {
                break;
            }
            if self.wait_read(1, timeout) {
                let n = self.pop_slice(slice);
                slice = &mut slice[n..];
                count += n;
            }
        }
        count
    }
}

pub struct PopAllIter<'a, C: Consumer> {
    pub(crate) target: &'a mut C,
    pub(crate) timeout: TimeoutIterator,
}

impl<'a, C: BlockingConsumer> Iterator for PopAllIter<'a, C> {
    type Item = C::Item;
    fn next(&mut self) -> Option<Self::Item> {
        let timeout = self.timeout.next()?;
        if self.target.wait_read(1, timeout) {
            self.target.try_pop()
        } else {
            None
        }
    }
}

impl<S: Storage, R: Index, W: Index> BlockingConsumer for Rb<S, BlockingIndex<R>, W> {
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        unsafe { self.read_index_ref() }
            .sem
            .wait(|| self.occupied_len() >= count, timeout)
    }
}

impl<R: Deref> BlockingConsumer for Cons<R>
where
    R::Target: RingBuffer + BlockingConsumer,
{
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base().wait_read(count, timeout)
    }
}
