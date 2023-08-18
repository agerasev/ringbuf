use crate::sync::{Instant, TimeoutIterator};
use core::time::Duration;

pub use ringbuf::traits::*;

pub trait BlockingProducer: Producer {
    type Instant: Instant;

    fn wait_vacant(&self, count: usize, timeout: Option<Duration>) -> bool;

    fn set_timeout(&mut self, timeout: Option<Duration>);
    fn timeout(&self) -> Option<Duration>;

    fn push(&mut self, item: Self::Item) -> Result<(), Self::Item> {
        if self.wait_vacant(1, self.timeout()) {
            assert!(self.try_push(item).is_ok());
            Ok(())
        } else {
            Err(item)
        }
    }

    fn push_iter_all<I: Iterator<Item = Self::Item>>(&mut self, iter: I) -> usize {
        let mut count = 0;
        let mut iter = iter.peekable();
        for timeout in TimeoutIterator::<Self::Instant>::new(self.timeout()) {
            if iter.peek().is_none() {
                break;
            }
            if self.wait_vacant(1, timeout) {
                count += self.push_iter(&mut iter);
            }
        }
        count
    }

    fn push_slice_all(&mut self, mut slice: &[Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        let mut count = 0;
        for timeout in TimeoutIterator::<Self::Instant>::new(self.timeout()) {
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

pub trait BlockingConsumer: Consumer {
    type Instant: Instant;

    fn wait_occupied(&self, count: usize, timeout: Option<Duration>) -> bool;

    fn set_timeout(&mut self, timeout: Option<Duration>);
    fn timeout(&self) -> Option<Duration>;

    fn pop_wait(&mut self) -> Option<Self::Item> {
        if self.wait_occupied(1, self.timeout()) {
            Some(self.try_pop().unwrap())
        } else {
            None
        }
    }

    fn pop_iter_all(&mut self) -> PopAllIter<'_, Self> {
        PopAllIter::new(self, self.timeout())
    }

    fn pop_slice_all(&mut self, mut slice: &mut [Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        let mut count = 0;
        for timeout in TimeoutIterator::<Self::Instant>::new(self.timeout()) {
            if slice.is_empty() {
                break;
            }
            if self.wait_occupied(1, timeout) {
                let n = self.pop_slice(slice);
                slice = &mut slice[n..];
                count += n;
            }
        }
        count
    }
}

pub struct PopAllIter<'a, C: BlockingConsumer> {
    pub(crate) target: &'a mut C,
    pub(crate) timeout: TimeoutIterator<C::Instant>,
}
impl<'a, C: BlockingConsumer> PopAllIter<'a, C> {
    pub fn new(target: &'a mut C, timeout: Option<Duration>) -> Self {
        Self {
            target,
            timeout: TimeoutIterator::new(timeout),
        }
    }
}
impl<'a, C: BlockingConsumer> Iterator for PopAllIter<'a, C> {
    type Item = C::Item;
    fn next(&mut self) -> Option<Self::Item> {
        let timeout = self.timeout.next()?;
        if self.target.wait_occupied(1, timeout) {
            self.target.try_pop()
        } else {
            None
        }
    }
}
