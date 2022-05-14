use crate::{consumer::AsyncConsumer, counter::AsyncCounter, producer::AsyncProducer};
use core::{marker::PhantomData, mem::MaybeUninit};
use ringbuf::{OwningRingBuffer, RingBuffer};

#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

pub struct AsyncRingBuffer<T, B>
where
    B: RingBuffer<T, Counter = AsyncCounter>,
{
    base: B,
    _phantom: PhantomData<T>,
}

impl<T, B> RingBuffer<T> for AsyncRingBuffer<T, B>
where
    B: RingBuffer<T, Counter = AsyncCounter>,
{
    type Counter = AsyncCounter;

    #[inline]
    fn capacity(&self) -> usize {
        self.base.capacity()
    }

    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.base.data()
    }

    #[inline]
    fn counter(&self) -> &AsyncCounter {
        self.base.counter()
    }
}

impl<T, B> AsyncRingBuffer<T, B>
where
    B: RingBuffer<T, Counter = AsyncCounter>,
{
    pub fn from_sync(base: B) -> Self {
        Self {
            base,
            _phantom: PhantomData,
        }
    }

    #[cfg(feature = "alloc")]
    pub fn split_async(self) -> (AsyncProducer<T, Arc<Self>>, AsyncConsumer<T, Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (AsyncProducer::new(arc.clone()), AsyncConsumer::new(arc)) }
    }

    pub fn split_async_static(&mut self) -> (AsyncProducer<T, &Self>, AsyncConsumer<T, &Self>) {
        unsafe { (AsyncProducer::new(self), AsyncConsumer::new(self)) }
    }
}

#[cfg(feature = "alloc")]
pub type AsyncHeapRingBuffer<T> =
    AsyncRingBuffer<T, OwningRingBuffer<T, Vec<MaybeUninit<T>>, AsyncCounter>>;

#[cfg(feature = "alloc")]
impl<T> AsyncHeapRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self::from_sync(OwningRingBuffer::new(capacity))
    }
}
