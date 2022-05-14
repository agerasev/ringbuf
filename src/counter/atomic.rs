use super::{Counter, DefaultCounter};
use cache_padded::CachePadded;
use core::{
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Thread-safe counter.
///
/// May be concurrently modified by two threads where one thread calls `set_head` and other calls `set_tail`.
pub struct AtomicCounter {
    len: NonZeroUsize,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl AtomicCounter {
    /// Construct new counter from capacity and head and tail positions.
    ///
    /// Panics if `tail - head` modulo `2 * len` is greater than `len`.
    pub fn new(len: NonZeroUsize, head: usize, tail: usize) -> Self {
        let self_ = Self {
            len,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        };
        assert!(self_.occupied_len() <= len.get());
        self_
    }
}

impl Counter for AtomicCounter {
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

impl DefaultCounter for AtomicCounter {
    fn with_capacity(len: NonZeroUsize) -> Self {
        Self::new(len, 0, 0)
    }
}
