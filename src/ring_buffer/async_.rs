use crate::{
    consumer::GlobalAsyncConsumer,
    producer::GlobalAsyncProducer,
    ring_buffer::{AbstractRingBuffer, BasicRingBuffer, Container, GlobalCounter},
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

pub struct BasicAsyncRingBuffer<T, C: Container<T>> {
    basic: BasicRingBuffer<T, C>,
    wakers: Wakers,
}

impl<T, C: Container<T>> AbstractRingBuffer<T> for BasicAsyncRingBuffer<T, C> {
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

impl<T, C: Container<T>> AbstractAsyncRingBuffer<T> for BasicAsyncRingBuffer<T, C> {
    #[inline]
    unsafe fn wakers(&self) -> &Wakers {
        &self.wakers
    }
}

impl<T, C: Container<T>> BasicAsyncRingBuffer<T, C> {
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            basic: BasicRingBuffer::from_raw_parts(container, head, tail),
            wakers: Wakers::default(),
        }
    }

    #[cfg(feature = "alloc")]
    pub fn split(
        self,
    ) -> (
        GlobalAsyncProducer<T, Self, Arc<Self>>,
        GlobalAsyncConsumer<T, Self, Arc<Self>>,
    ) {
        let arc = Arc::new(self);
        unsafe {
            (
                GlobalAsyncProducer::new(arc.clone()),
                GlobalAsyncConsumer::new(arc),
            )
        }
    }

    pub fn split_static(
        &mut self,
    ) -> (
        GlobalAsyncProducer<T, Self, &Self>,
        GlobalAsyncConsumer<T, Self, &Self>,
    ) {
        unsafe {
            (
                GlobalAsyncProducer::new(self),
                GlobalAsyncConsumer::new(self),
            )
        }
    }
}
