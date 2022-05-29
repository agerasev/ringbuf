//use crate::{consumer::AsyncConsumer, producer::AsyncProducer};
use core::num::NonZeroUsize;
use futures::task::AtomicWaker;
use ringbuf::ring_buffer::{Rb, RbBase, RbRead, RbWrite};

#[derive(Default)]
pub struct Wakers {
    pub head: AtomicWaker,
    pub tail: AtomicWaker,
}

pub struct AsyncRb<T, B: Rb<T>> {
    base: B,
    wakers: Wakers,
}

impl<T, B: Rb<T>> AsyncRb<T, B> {
    fn new(base: B) -> Self {
        Self {
            base,
            wakers: Wakers::default(),
        }
    }

    pub fn wakers(&self) -> &Wakers {
        &self.wakers
    }
}

impl<T, B: Rb<T>> RbBase<T> for AsyncRb<T, B> {
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    fn head(&self) -> usize {
        self.base.head()
    }

    fn tail(&self) -> usize {
        self.base.tail()
    }
}

impl<T, B: Rb<T>> RbRead<T> for AsyncRb<T, B> {
    unsafe fn set_head(&self, value: usize) {
        self.base.set_head(value);
        self.wakers.head.wake();
    }
}

impl<T, B: Rb<T>> RbWrite<T> for AsyncRb<T, B> {
    unsafe fn set_tail(&self, value: usize) {
        self.base.set_tail(value);
        self.wakers.tail.wake();
    }
}

impl<T, B: Rb<T>> Rb<T> for AsyncRb<T, B> {}
/*
impl<T, B: Rb<T>> AsyncRb<T, B> {
    #[cfg(feature = "alloc")]
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
*/
