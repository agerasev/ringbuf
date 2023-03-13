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

#[derive(Default)]
struct Semaphore {
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl Semaphore {
    fn wait<F: Fn() -> bool>(&self, f: F, timeout: Option<Duration>) -> bool {
        let instant = Instant::now();
        let mut guard_slot = Some(self.mutex.lock().unwrap());
        loop {
            if f() {
                break true;
            }
            let guard = guard_slot.take().unwrap();
            guard_slot.replace(match timeout {
                Some(dur) => {
                    let elapsed = instant.elapsed();
                    if dur > elapsed {
                        self.condvar.wait_timeout(guard, dur - elapsed).unwrap().0
                    } else {
                        break false;
                    }
                }
                None => self.condvar.wait(guard).unwrap(),
            });
        }
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
        self.read_sem.wait(
            || {
                println!("self.len() = {}", self.len());
                self.len() >= count
            },
            timeout,
        )
    }
    fn wait_write(&self, count: usize, timeout: Option<Duration>) -> bool {
        assert!(count <= self.capacity());
        self.write_sem.wait(
            || {
                println!("self.free_len() = {}", self.free_len());
                self.free_len() >= count
            },
            timeout,
        )
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
    unsafe fn new(target: R) -> Self {
        BasicProducer::new(target).into()
    }

    pub fn wait(&self, count: usize) {
        self.rb().wait_write(count, None);
    }
    pub fn wait_timeout(&self, count: usize, timeout: Duration) -> bool {
        self.rb().wait_write(count, Some(timeout))
    }
}

#[derive(Deref, DerefMut, From, Into)]
pub struct Consumer<T, R: RbRef>(BasicConsumer<T, R>)
where
    R::Rb: RbRead<T>;

impl<T, B: BasicRb<T>, R: RbRef<Rb = Rb<T, B>>> Consumer<T, R> {
    unsafe fn new(target: R) -> Self {
        BasicConsumer::new(target).into()
    }

    pub fn wait(&self, count: usize) {
        self.rb().wait_read(count, None);
    }
    pub fn wait_timeout(&self, count: usize, timeout: Duration) -> bool {
        self.rb().wait_read(count, Some(timeout))
    }
}
