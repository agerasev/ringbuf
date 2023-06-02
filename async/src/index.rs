use core::sync::atomic::{AtomicBool, Ordering};
use futures::task::AtomicWaker;
use ringbuf::index::Index;

#[derive(Default)]
pub struct AsyncIndex<I: Index> {
    base: I,
    pub(crate) waker: AtomicWaker,
    closed: AtomicBool,
}

impl<I: Index> Index for AsyncIndex<I> {
    #[inline]
    fn get(&self) -> usize {
        self.base.get()
    }
    fn set(&self, value: usize) {
        self.base.set(value);
        self.waker.wake();
    }
}

impl<I: Index> AsyncIndex<I> {
    pub(crate) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.waker.wake();
        std::println!("AsyncIndex::close");
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

impl<I: Index> Drop for AsyncIndex<I> {
    fn drop(&mut self) {
        self.close();
    }
}
