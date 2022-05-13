use core::num::NonZeroUsize;
use futures::task::AtomicWaker;
use ringbuf::counter::{AtomicCounter, Counter};

#[derive(Default)]
pub struct Wakers {
    pub head: AtomicWaker,
    pub tail: AtomicWaker,
}

pub struct AsyncCounter {
    base: AtomicCounter,
    wakers: Wakers,
}

impl AsyncCounter {
    pub fn wakers(&self) -> &Wakers {
        &self.wakers
    }
}

impl Counter for AsyncCounter {
    fn new(len: NonZeroUsize, head: usize, tail: usize) -> Self {
        Self {
            base: AtomicCounter::new(len, head, tail),
            wakers: Wakers::default(),
        }
    }

    fn len(&self) -> NonZeroUsize {
        self.base.len()
    }
    fn head(&self) -> usize {
        self.base.head()
    }
    fn tail(&self) -> usize {
        self.base.tail()
    }

    unsafe fn set_head(&self, value: usize) {
        self.base.set_head(value);
        self.wakers.head.wake();
    }
    unsafe fn set_tail(&self, value: usize) {
        self.base.set_tail(value);
        self.wakers.tail.wake();
    }
}
