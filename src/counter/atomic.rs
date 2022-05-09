use super::Counter;
use cache_padded::CachePadded;
use core::{
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct AtomicCounter {
    len: NonZeroUsize,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl Counter for AtomicCounter {
    fn new(len: NonZeroUsize, head: usize, tail: usize) -> Self {
        Self {
            len,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    fn len(&self) -> NonZeroUsize {
        self.len
    }
    fn head(&self) -> usize {
        self.head.load(Ordering::Acquire)
    }
    fn tail(&self) -> usize {
        self.tail.load(Ordering::Acquire)
    }

    unsafe fn set_head(&self, value: usize) {
        self.head.store(value, Ordering::Release)
    }
    unsafe fn set_tail(&self, value: usize) {
        self.tail.store(value, Ordering::Release)
    }
}
