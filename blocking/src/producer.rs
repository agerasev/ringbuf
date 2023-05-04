use crate::{index::BlockingIndex, utils::TimeoutIterator};
use ringbuf::{
    index::Index,
    storage::Storage,
    traits::{Observer, Producer, RingBuffer},
    Prod, Rb,
};
use std::{ops::Deref, time::Duration};

pub trait BlockingProducer: Producer {
    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool;

    fn push(&mut self, item: Self::Item, timeout: Option<Duration>) -> Result<(), Self::Item> {
        if self.wait_vacant(1, timeout) {
            assert!(self.try_push(item).is_ok());
            Ok(())
        } else {
            Err(item)
        }
    }

    fn push_iter_all<I: Iterator<Item = Self::Item>>(
        &mut self,
        iter: I,
        timeout: Option<Duration>,
    ) -> usize {
        let mut count = 0;
        let mut iter = iter.peekable();
        for timeout in TimeoutIterator::new(timeout) {
            if iter.peek().is_none() {
                break;
            }
            if self.wait_vacant(1, timeout) {
                count += self.push_iter(&mut iter);
            }
        }
        count
    }

    fn push_slice_all(&mut self, mut slice: &[Self::Item], timeout: Option<Duration>) -> usize
    where
        Self::Item: Copy,
    {
        let mut count = 0;
        for timeout in TimeoutIterator::new(timeout) {
            if slice.is_empty() {
                break;
            }
            if self.wait_vacant(1, timeout) {
                let n = self.push_slice(slice);
                slice = &slice[n..];
                count += n;
            }
        }
        count
    }
}

impl<S: Storage, R: Index, W: Index> BlockingProducer for Rb<S, BlockingIndex<R>, W> {
    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        unsafe { self.read_index_ref() }
            .sem
            .wait(|| self.vacant_len() >= count, timeout)
    }
}

impl<R: Deref> BlockingProducer for Prod<R>
where
    R::Target: RingBuffer + BlockingProducer,
{
    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base().wait_vacant(count, timeout)
    }
}
