use core::{
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbeam_utils::CachePadded;

pub trait Index {
    fn get(&self) -> usize;
    fn set(&self, value: usize);
}

#[derive(Default)]
pub struct LocalIndex {
    value: Cell<usize>,
}

impl Index for LocalIndex {
    fn get(&self) -> usize {
        self.value.get()
    }
    fn set(&self, value: usize) {
        self.value.set(value);
    }
}

#[derive(Default)]
pub struct SharedIndex {
    value: CachePadded<AtomicUsize>,
}

impl Index for SharedIndex {
    fn get(&self) -> usize {
        self.value.load(Ordering::Acquire)
    }
    fn set(&self, value: usize) {
        self.value.store(value, Ordering::Release);
    }
}
