use crate::{
    traits::BlockingConsumer,
    utils::{Semaphore, TimeoutIterator},
};
use ringbuf::{
    traits::{Consumer, RingBuffer},
    Cons, HeapRb, Prod,
};
use std::sync::Arc;

pub struct BlockingRb<B: RingBuffer> {
    pub(crate) base: B,
    pub(crate) read_sem: Semaphore,
    pub(crate) write_sem: Semaphore,
}

impl<B: RingBuffer> From<B> for BlockingRb<B> {
    fn from(base: B) -> Self {
        Self {
            base,
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
        }
    }
}
impl<B: RingBuffer> BlockingRb<B> {
    pub fn into_inner(self) -> B {
        self.base
    }
}

impl<B: RingBuffer + Default> Default for BlockingRb<B> {
    fn default() -> Self {
        Self {
            base: B::default(),
            read_sem: Semaphore::default(),
            write_sem: Semaphore::default(),
        }
    }
}

impl<T> BlockingRb<HeapRb<T>> {
    pub fn new(capacity: usize) -> Self {
        Self::from(HeapRb::new(capacity))
    }
    pub fn split(self) -> (Prod<Arc<Self>>, Cons<Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (Prod::new(arc.clone()), Cons::new(arc)) }
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
