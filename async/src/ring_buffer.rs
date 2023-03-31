#![allow(clippy::missing_safety_doc)]

use crate::{consumer::AsyncConsumer, producer::AsyncProducer};
use core::{
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZeroUsize,
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};
use futures::task::AtomicWaker;
use ringbuf::{
    ring_buffer::{Rb, RbBase, RbRead, RbWrite},
    Consumer, Producer,
};

#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};
#[cfg(feature = "alloc")]
use ringbuf::ring_buffer::{Container, LocalRb, SharedRb};
#[cfg(feature = "alloc")]
use ringbuf::HeapRb;

#[derive(Default)]
pub struct Wakers {
    pub head: AtomicWaker,
    pub tail: AtomicWaker,
}

pub trait AsyncRbBase<T>: RbBase<T> {
    fn is_closed(&self) -> bool;
}
pub trait AsyncRbRead<T>: RbRead<T> + AsyncRbBase<T> {
    unsafe fn register_tail_waker(&self, waker: &Waker);
    unsafe fn close_head(&self);
}
pub trait AsyncRbWrite<T>: RbWrite<T> + AsyncRbBase<T> {
    unsafe fn register_head_waker(&self, waker: &Waker);
    unsafe fn close_tail(&self);
}

pub struct AsyncRb<T, B: Rb<T>> {
    base: B,
    wakers: Wakers,
    closed: AtomicBool,
    _phantom: PhantomData<T>,
}

impl<T, B: Rb<T>> RbBase<T> for AsyncRb<T, B> {
    unsafe fn slices(
        &self,
        head: usize,
        tail: usize,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.base.slices(head, tail)
    }

    fn capacity_nonzero(&self) -> NonZeroUsize {
        self.base.capacity_nonzero()
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

impl<T, B: Rb<T>> AsyncRbBase<T> for AsyncRb<T, B> {
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}
impl<T, B: Rb<T>> AsyncRbRead<T> for AsyncRb<T, B> {
    unsafe fn register_tail_waker(&self, waker: &Waker) {
        self.wakers.tail.register(waker);
    }
    unsafe fn close_head(&self) {
        self.closed.store(true, Ordering::Release);
        self.wakers.head.wake();
    }
}
impl<T, B: Rb<T>> AsyncRbWrite<T> for AsyncRb<T, B> {
    unsafe fn register_head_waker(&self, waker: &Waker) {
        self.wakers.head.register(waker);
    }
    unsafe fn close_tail(&self) {
        self.closed.store(true, Ordering::Release);
        self.wakers.tail.wake();
    }
}

impl<T, B: Rb<T>> Rb<T> for AsyncRb<T, B> {}

impl<T, B: Rb<T>> AsyncRb<T, B> {
    pub fn from_base(base: B) -> Self {
        Self {
            base,
            wakers: Wakers::default(),
            closed: AtomicBool::new(false),
            _phantom: PhantomData,
        }
    }

    pub fn into_base(self) -> B {
        self.base
    }

    pub fn split_ref(&mut self) -> (AsyncProducer<T, &Self>, AsyncConsumer<T, &Self>) {
        unsafe {
            (
                AsyncProducer::from_base(Producer::new(self)),
                AsyncConsumer::from_base(Consumer::new(self)),
            )
        }
    }
}

#[cfg(feature = "alloc")]
impl<T, C: Container<T>> AsyncRb<T, LocalRb<T, C>> {
    pub fn split(self) -> (AsyncProducer<T, Rc<Self>>, AsyncConsumer<T, Rc<Self>>) {
        let rc = Rc::new(self);
        unsafe {
            (
                AsyncProducer::from_base(Producer::new(rc.clone())),
                AsyncConsumer::from_base(Consumer::new(rc)),
            )
        }
    }
}

#[cfg(feature = "alloc")]
impl<T, C: Container<T>> AsyncRb<T, SharedRb<T, C>> {
    pub fn split(self) -> (AsyncProducer<T, Arc<Self>>, AsyncConsumer<T, Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe {
            (
                AsyncProducer::from_base(Producer::new(arc.clone())),
                AsyncConsumer::from_base(Consumer::new(arc)),
            )
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> AsyncRb<T, HeapRb<T>> {
    pub fn new(capacity: usize) -> Self {
        Self::from_base(HeapRb::new(capacity))
    }
}
