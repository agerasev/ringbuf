use crate::{rb::PopAllIter, utils::TimeoutIterator, BlockingRb};
use ringbuf::{
    traits::{Consumer, Observer, Producer, RingBuffer},
    Cons, Prod,
};
use std::{mem::MaybeUninit, num::NonZeroUsize, ops::Deref, time::Duration};

impl<B: RingBuffer> Observer for BlockingRb<B> {
    type Item = B::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }
    #[inline]
    fn read_index(&self) -> usize {
        self.base.read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.base.write_index()
    }
}

impl<B: RingBuffer> Consumer for BlockingRb<B> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.write_sem.notify(|| self.base.set_read_index(value));
    }
    #[inline]
    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        self.base.occupied_slices()
    }
    #[inline]
    unsafe fn occupied_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.occupied_slices_mut()
    }
}

impl<B: RingBuffer> Producer for BlockingRb<B> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.read_sem.notify(|| self.base.set_write_index(value));
    }
    #[inline]
    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        self.base.vacant_slices()
    }
    #[inline]
    fn vacant_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.vacant_slices_mut()
    }
}

impl<B: RingBuffer> RingBuffer for BlockingRb<B> {
    #[inline]
    unsafe fn unsafe_slices(
        &self,
        start: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.unsafe_slices(start, end)
    }
}

pub trait BlockingProducer: Producer {
    fn wait_write(&self, count: usize, timeout: Option<Duration>) -> bool;

    fn push(&mut self, item: Self::Item, timeout: Option<Duration>) -> Result<(), Self::Item> {
        if self.wait_write(1, timeout) {
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
            if self.wait_write(1, timeout) {
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
            if self.wait_write(1, timeout) {
                let n = self.push_slice(slice);
                slice = &slice[n..];
                count += n;
            }
        }
        count
    }
}

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

pub trait BlockingRingBuffer: RingBuffer + BlockingProducer + BlockingConsumer {}

impl<B: RingBuffer> BlockingProducer for BlockingRb<B> {
    fn wait_write(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.write_sem.wait(|| self.vacant_len() >= count, timeout)
    }
}
impl<B: RingBuffer> BlockingConsumer for BlockingRb<B> {
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool {
        debug_assert!(count <= self.capacity().get());
        self.read_sem.wait(|| self.occupied_len() >= count, timeout)
    }
}
impl<B: RingBuffer> BlockingRingBuffer for BlockingRb<B> {}

impl<R: Deref> BlockingProducer for Prod<R>
where
    R::Target: BlockingRingBuffer,
{
    fn wait_write(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base().wait_write(count, timeout)
    }
}
impl<R: Deref> BlockingConsumer for Cons<R>
where
    R::Target: BlockingRingBuffer,
{
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.base().wait_read(count, timeout)
    }
}
