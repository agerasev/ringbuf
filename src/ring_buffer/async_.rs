use crate::ring_buffer::{BasicRingBuffer, Container, Storage};
use core::{
    future::Future,
    sync::atomic::AtomicBool,
    task::{Context, Poll, Waker},
};
use futures::task::AtomicWaker;

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

pub struct AsyncRingBuffer<T, C: Container<T>> {
    basic: BasicRingBuffer<T, C>,
    push_waker: AtomicWaker,
    pop_waker: AtomicWaker,
}

impl<T, C: Container<T>> AsyncRingBuffer<T, C> {
    #[inline]
    pub fn capacity(&self) -> usize {
        self.basic.capacity()
    }

    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            basic: BasicRingBuffer::from_raw_parts(container, head, tail),
            push_waker: AtomicWaker::new(),
            pop_waker: AtomicWaker::new(),
        }
    }
}
