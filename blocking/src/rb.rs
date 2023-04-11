use crate::{
    raw::RawBlockingRb,
    utils::{Semaphore, TimeoutIterator},
};
use ringbuf::{
    raw::{AsRaw, ConsMarker, ProdMarker, RawRb, RbMarker},
    traits::{Consumer, Observer, Producer, RingBuffer},
    Cons, HeapRb, Prod,
};
use std::{mem::MaybeUninit, num::NonZeroUsize, sync::Arc, time::Duration};

pub struct BlockingRb<B: RawRb> {
    inner: B,
    read_sem: Semaphore,
    write_sem: Semaphore,
}

impl<B: RawRb> From<B> for BlockingRb<B> {
    fn from(inner: B) -> Self {
        Self {
            inner,
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
        }
    }
}
impl<B: RawRb> BlockingRb<B> {
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: RawRb + Default> Default for BlockingRb<B> {
    fn default() -> Self {
        Self {
            inner: B::default(),
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
        }
    }
}

impl<B: RawRb> RawRb for BlockingRb<B> {
    type Item = B::Item;

    #[inline]
    unsafe fn slices(
        &self,
        begin: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.inner.slices(begin, end)
    }
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.inner.capacity()
    }
    #[inline]
    fn read_index(&self) -> usize {
        self.inner.read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.inner.write_index()
    }
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.write_sem.notify(|| self.inner.set_read_index(value));
    }
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.read_sem.notify(|| self.inner.set_write_index(value));
    }
}

impl<B: RawRb> RawBlockingRb for BlockingRb<B> {
    fn wait_write<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool {
        self.write_sem.wait(pred, timeout)
    }
    fn wait_read<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool {
        self.read_sem.wait(pred, timeout)
    }
}

impl<B: RawRb> AsRaw for BlockingRb<B> {
    type Raw = Self;
    fn as_raw(&self) -> &Self::Raw {
        self
    }
}
unsafe impl<B: RawRb> RbMarker for BlockingRb<B> {}

impl<T> BlockingRb<HeapRb<T>> {
    pub fn new(capacity: usize) -> Self {
        Self::from(HeapRb::new(capacity))
    }
    pub fn split(self) -> (Prod<Arc<Self>>, Cons<Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (Prod::new(arc.clone()), Cons::new(arc)) }
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

pub struct PopAllIter<'a, C: Consumer> {
    target: &'a mut C,
    timeout: TimeoutIterator,
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

pub trait BlockingRingBuffer: RingBuffer {}

impl<R: AsRaw + ProdMarker> BlockingProducer for R
where
    R::Raw: RawBlockingRb,
{
    fn wait_write(&self, count: usize, timeout: Option<Duration>) -> bool {
        assert!(count <= self.capacity());
        self.as_raw()
            .wait_write(|| self.vacant_len() >= count, timeout)
    }
}

impl<R: AsRaw + ConsMarker> BlockingConsumer for R
where
    R::Raw: RawBlockingRb,
{
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool {
        assert!(count <= self.capacity());
        self.as_raw()
            .wait_read(|| self.occupied_len() >= count, timeout)
    }
}
impl<R: AsRaw + RbMarker> BlockingRingBuffer for R where R::Raw: RawBlockingRb {}
