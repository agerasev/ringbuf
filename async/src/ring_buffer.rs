use crate::{consumer::AsyncConsumer, counter::AsyncCounter, producer::AsyncProducer};
use ringbuf::{Container, RingBuffer};

pub struct AsyncRingBuffer<T, C: Container<T>> {
    base: RingBuffer<T, C, AsyncCounter>,
}

impl<T, C: Container> AsyncRingBuffer<T, C> {
    pub fn from_sync(base: RingBuffer<T, C, AsyncCounter>) -> Self {}
}

impl<T, C: Container<T>> RingBuffer<T, C, AsyncCounter> {
    #[cfg(feature = "alloc")]
    #[allow(clippy::type_complexity)]
    pub fn split_async(
        self,
    ) -> (
        AsyncProducer<T, C, Arc<Self>>,
        AsyncConsumer<T, C, Arc<Self>>,
    ) {
        let arc = Arc::new(self);
        unsafe { (AsyncProducer::new(arc.clone()), AsyncConsumer::new(arc)) }
    }

    pub fn split_async_static(
        &mut self,
    ) -> (AsyncProducer<T, C, &Self>, AsyncConsumer<T, C, &Self>) {
        unsafe { (AsyncProducer::new(self), AsyncConsumer::new(self)) }
    }
}

#[cfg(feature = "alloc")]
pub type AsyncHeapRingBuffer<T> = RingBuffer<T, Vec<MaybeUninit<T>>, AsyncCounter>;

#[cfg(feature = "alloc")]
impl<T> AsyncHeapRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}
