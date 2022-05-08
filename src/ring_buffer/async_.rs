use crate::{
    consumer::AsyncConsumer,
    producer::AsyncProducer,
    ring_buffer::{AbstractRingBuffer, Container, GlobalCounter, RingBuffer},
};
use core::{
    future::Future,
    mem::MaybeUninit,
    num::NonZeroUsize,
    task::{Context, Poll, Waker},
};
use futures::task::AtomicWaker;

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

#[derive(Default)]
pub struct Wakers {
    pub push: AtomicWaker,
    pub pop: AtomicWaker,
}

pub trait AbstractAsyncRingBuffer<T>: AbstractRingBuffer<T> {
    unsafe fn wakers(&self) -> &Wakers;
}

pub struct AsyncRingBuffer<T, C: Container<T>> {
    basic: RingBuffer<T, C>,
    wakers: Wakers,
}

impl<T, C: Container<T>> AbstractRingBuffer<T> for AsyncRingBuffer<T, C> {
    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.basic.capacity()
    }
    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.basic.data()
    }
    #[inline]
    fn counter(&self) -> &GlobalCounter {
        self.basic.counter()
    }
}

impl<T, C: Container<T>> AbstractAsyncRingBuffer<T> for AsyncRingBuffer<T, C> {
    #[inline]
    unsafe fn wakers(&self) -> &Wakers {
        &self.wakers
    }
}

impl<T, C: Container<T>> AsyncRingBuffer<T, C> {
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            basic: RingBuffer::from_raw_parts(container, head, tail),
            wakers: Wakers::default(),
        }
    }

    #[cfg(feature = "alloc")]
    pub fn split(
        self,
    ) -> (
        AsyncProducer<T, Self, Arc<Self>>,
        AsyncConsumer<T, Self, Arc<Self>>,
    ) {
        let arc = Arc::new(self);
        unsafe { (AsyncProducer::new(arc.clone()), AsyncConsumer::new(arc)) }
    }

    pub fn split_static(
        &mut self,
    ) -> (AsyncProducer<T, Self, &Self>, AsyncConsumer<T, Self, &Self>) {
        unsafe { (AsyncProducer::new(self), AsyncConsumer::new(self)) }
    }
}
