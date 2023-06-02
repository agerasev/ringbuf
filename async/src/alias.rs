use crate::rb::AsyncRb;
use ringbuf::HeapRb;

pub type AsyncHeapRb<T> = AsyncRb<HeapRb<T>>;

impl<T> AsyncHeapRb<T> {
    pub fn new(cap: usize) -> Self {
        Self::from(HeapRb::new(cap))
    }
}
