#[cfg(test)]
mod tests;

use derive_more::{Deref, DerefMut, From, Into};
use ringbuf::{
    ring_buffer::{RbBase, RbRead, RbRef, RbWrite},
    Consumer as BasicConsumer, HeapRb, Producer as BasicProducer, Rb as BasicRb,
};
use std::{
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZeroUsize,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
struct TimeoutIterator {
    start: Instant,
    timeout: Option<Duration>,
}

impl TimeoutIterator {
    fn new(timeout: Option<Duration>) -> Self {
        Self {
            start: Instant::now(),
            timeout,
        }
    }
}

impl Iterator for TimeoutIterator {
    type Item = Option<Duration>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.timeout {
            Some(dur) => {
                let elapsed = self.start.elapsed();
                if dur > elapsed {
                    Some(Some(dur - elapsed))
                } else {
                    None
                }
            }
            None => Some(None),
        }
    }
}

#[derive(Default)]
struct Semaphore {
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl Semaphore {
    fn wait<F: Fn() -> bool>(&self, f: F, timeout: Option<Duration>) -> bool {
        let mut guard_slot = Some(self.mutex.lock().unwrap());
        for timeout in TimeoutIterator::new(timeout) {
            if f() {
                return true;
            }
            let guard = guard_slot.take().unwrap();
            guard_slot.replace(match timeout {
                Some(t) => self.condvar.wait_timeout(guard, t).unwrap().0,
                None => self.condvar.wait(guard).unwrap(),
            });
        }
        f()
    }
    fn notify<F: FnOnce()>(&self, f: F) {
        let _guard = self.mutex.lock();
        f();
        self.condvar.notify_one();
    }
}

#[derive(Deref, DerefMut)]
pub struct Rb<T, B: BasicRb<T>> {
    #[deref]
    #[deref_mut]
    inner: B,
    read_sem: Semaphore,
    write_sem: Semaphore,
    _p: PhantomData<T>,
}

impl<T, B: BasicRb<T>> From<B> for Rb<T, B> {
    fn from(inner: B) -> Self {
        Self {
            inner,
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
            _p: PhantomData,
        }
    }
}
impl<T, B: BasicRb<T>> Rb<T, B> {
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<T, B: BasicRb<T> + Default> Default for Rb<T, B> {
    fn default() -> Self {
        Self {
            inner: B::default(),
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
            _p: PhantomData,
        }
    }
}

impl<T, B: BasicRb<T>> RbBase<T> for Rb<T, B> {
    #[inline]
    unsafe fn slices(
        &self,
        head: usize,
        tail: usize,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.inner.slices(head, tail)
    }
    #[inline]
    fn capacity_nonzero(&self) -> NonZeroUsize {
        self.inner.capacity_nonzero()
    }
    #[inline]
    fn head(&self) -> usize {
        self.inner.head()
    }
    #[inline]
    fn tail(&self) -> usize {
        self.inner.tail()
    }
}

impl<T, B: BasicRb<T>> RbRead<T> for Rb<T, B> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.write_sem.notify(|| self.inner.set_head(value));
    }
}

impl<T, B: BasicRb<T>> RbWrite<T> for Rb<T, B> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.read_sem.notify(|| self.inner.set_tail(value));
    }
}

impl<T, B: BasicRb<T>> BasicRb<T> for Rb<T, B> {}

impl<T, B: BasicRb<T>> Rb<T, B> {
    fn wait_read(&self, count: usize, timeout: Option<Duration>) -> bool {
        assert!(count <= self.capacity());
        self.read_sem.wait(|| self.len() >= count, timeout)
    }
    fn wait_write(&self, count: usize, timeout: Option<Duration>) -> bool {
        assert!(count <= self.capacity());
        self.write_sem.wait(|| self.free_len() >= count, timeout)
    }
}

impl<T> Rb<T, HeapRb<T>> {
    pub fn new(capacity: usize) -> Self {
        Self::from(HeapRb::new(capacity))
    }
    pub fn split(self) -> (Producer<T, Arc<Self>>, Consumer<T, Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (Producer::new(arc.clone()), Consumer::new(arc)) }
    }
}

#[derive(Deref, DerefMut, From, Into)]
pub struct Producer<T, R: RbRef>(BasicProducer<T, R>)
where
    R::Rb: RbWrite<T>;

impl<T, B: BasicRb<T>, R: RbRef<Rb = Rb<T, B>>> Producer<T, R> {
    pub unsafe fn new(target: R) -> Self {
        BasicProducer::new(target).into()
    }

    pub fn wait(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.rb().wait_write(count, timeout)
    }

    pub fn push_wait(&mut self, item: T, timeout: Option<Duration>) -> Result<(), T> {
        if self.wait(1, timeout) {
            assert!(self.push(item).is_ok());
            Ok(())
        } else {
            Err(item)
        }
    }
    pub fn push_iter_all<I: Iterator<Item = T>>(
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
            if self.wait(1, timeout) {
                count += self.push_iter(&mut iter);
            }
        }
        count
    }
}

impl<T: Copy, B: BasicRb<T>, R: RbRef<Rb = Rb<T, B>>> Producer<T, R> {
    pub fn push_slice_all(&mut self, mut slice: &[T], timeout: Option<Duration>) -> usize {
        let mut count = 0;
        for timeout in TimeoutIterator::new(timeout) {
            if slice.is_empty() {
                break;
            }
            if self.wait(1, timeout) {
                let n = self.push_slice(slice);
                slice = &slice[n..];
                count += n;
            }
        }
        count
    }
}

#[derive(Deref, DerefMut, From, Into)]
pub struct Consumer<T, R: RbRef>(BasicConsumer<T, R>)
where
    R::Rb: RbRead<T>;

impl<T, B: BasicRb<T>, R: RbRef<Rb = Rb<T, B>>> Consumer<T, R> {
    pub unsafe fn new(target: R) -> Self {
        BasicConsumer::new(target).into()
    }

    pub fn wait(&self, count: usize, timeout: Option<Duration>) -> bool {
        self.rb().wait_read(count, timeout)
    }

    pub fn pop_wait(&mut self, timeout: Option<Duration>) -> Option<T> {
        if self.wait(1, timeout) {
            Some(self.pop().unwrap())
        } else {
            None
        }
    }
    pub fn pop_iter_all(&mut self, timeout: Option<Duration>) -> impl Iterator<Item = T> + '_ {
        TimeoutIterator::new(timeout).map_while(|timeout| {
            if self.wait(1, timeout) {
                self.pop()
            } else {
                None
            }
        })
    }
}

impl<T: Copy, B: BasicRb<T>, R: RbRef<Rb = Rb<T, B>>> Consumer<T, R> {
    pub fn pop_slice_all(&mut self, mut slice: &mut [T], timeout: Option<Duration>) -> usize {
        let mut count = 0;
        for timeout in TimeoutIterator::new(timeout) {
            if slice.is_empty() {
                break;
            }
            if self.wait(1, timeout) {
                let n = self.pop_slice(slice);
                slice = &mut slice[n..];
                count += n;
            }
        }
        count
    }
}
