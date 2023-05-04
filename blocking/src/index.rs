use crate::utils::Semaphore;
use ringbuf::index::Index;

#[derive(Default)]
pub struct BlockingIndex<I: Index> {
    base: I,
    pub(crate) sem: Semaphore,
}

impl<I: Index> Index for BlockingIndex<I> {
    #[inline]
    fn get(&self) -> usize {
        self.base.get()
    }
    fn set(&self, value: usize) {
        self.sem.notify(|| self.base.set(value));
    }
}
