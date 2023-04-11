use crate::{
    raw::{RawCons, RawProd, RawRb},
    utils::{Semaphore, TimeoutIterator},
};
use ringbuf::{
    raw::{AsRaw, ConsMarker, ProdMarker, Raw, RbMarker},
    traits::Observer,
    Cons, HeapRb, Prod,
};
use std::{mem::MaybeUninit, num::NonZeroUsize, sync::Arc, time::Duration};

mod base {
    pub use ringbuf::{
        raw::{AsRaw, RawCons, RawProd, RawRb},
        traits::{Consumer, Producer, RingBuffer},
    };
}

pub struct Rb<B: base::RawRb> {
    inner: B,
    read_sem: Semaphore,
    write_sem: Semaphore,
}

impl<B: base::RawRb> From<B> for Rb<B> {
    fn from(inner: B) -> Self {
        Self {
            inner,
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
        }
    }
}
impl<B: base::RawRb> Rb<B> {
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: base::RawRb + Default> Default for Rb<B> {
    fn default() -> Self {
        Self {
            inner: B::default(),
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
        }
    }
}

impl<B: base::RawRb> Raw for Rb<B> {
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
    fn read_end(&self) -> usize {
        self.inner.read_end()
    }
    #[inline]
    fn write_end(&self) -> usize {
        self.inner.write_end()
    }
}
impl<B: base::RawRb> base::RawCons for Rb<B> {
    #[inline]
    unsafe fn set_read_end(&self, value: usize) {
        self.write_sem.notify(|| self.inner.set_read_end(value));
    }
}
impl<B: base::RawRb> base::RawProd for Rb<B> {
    #[inline]
    unsafe fn set_write_end(&self, value: usize) {
        self.read_sem.notify(|| self.inner.set_write_end(value));
    }
}
impl<B: base::RawRb> base::RawRb for Rb<B> {}

impl<B: base::RawRb> RawProd for Rb<B> {
    fn wait_write<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool {
        self.write_sem.wait(pred, timeout)
    }
}
impl<B: base::RawRb> RawCons for Rb<B> {
    fn wait_read<F: Fn() -> bool>(&self, pred: F, timeout: Option<Duration>) -> bool {
        self.read_sem.wait(pred, timeout)
    }
}
impl<B: base::RawRb> RawRb for Rb<B> {}

impl<B: base::RawRb> AsRaw for Rb<B> {
    type Raw = Self;
    fn as_raw(&self) -> &Self::Raw {
        self
    }
}
impl<B: base::RawRb> RbMarker for Rb<B> {}

impl<T> Rb<HeapRb<T>> {
    pub fn new(capacity: usize) -> Self {
        Self::from(HeapRb::new(capacity))
    }
    pub fn split(self) -> (Prod<Arc<Self>>, Cons<Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (Prod::new(arc.clone()), Cons::new(arc)) }
    }
}

pub trait Producer: base::Producer {
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

pub trait Consumer: base::Consumer {
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

pub struct PopAllIter<'a, C: Consumer + ?Sized> {
    target: &'a mut C,
    timeout: TimeoutIterator,
}
impl<'a, C: Consumer + ?Sized> Iterator for PopAllIter<'a, C> {
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

pub trait RingBuffer: base::RingBuffer {}

impl<R: AsRaw + ProdMarker> Producer for R
where
    R::Raw: RawProd,
{
    fn wait_write(&self, count: usize, timeout: Option<Duration>) -> bool {
        assert!(count <= self.capacity());
        self.as_raw()
            .wait_write(|| self.vacant_len() >= count, timeout)
    }
}

impl<R: AsRaw + ConsMarker> Consumer for R
where
    R::Raw: RawCons,
{
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool {
        assert!(count <= self.capacity());
        self.as_raw()
            .wait_read(|| self.occupied_len() >= count, timeout)
    }
}
