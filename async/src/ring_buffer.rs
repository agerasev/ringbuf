use crate::{consumer::AsyncConsumer, producer::AsyncProducer};
use core::{marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, task::Waker};
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

pub struct AsyncRb<T, B: Rb<T>> {
    base: B,
    wakers: Wakers,
    _phantom: PhantomData<T>,
}

impl<T, B: Rb<T>> RbBase<T> for AsyncRb<T, B> {
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.base.data()
    }

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

pub trait AsyncRbRead<T>: RbRead<T> {
    fn register_tail_waker(&self, waker: &Waker);
}
impl<T, B: Rb<T>> AsyncRbRead<T> for AsyncRb<T, B> {
    fn register_tail_waker(&self, waker: &Waker) {
        self.wakers.tail.register(waker);
    }
}

pub trait AsyncRbWrite<T>: RbWrite<T> {
    fn register_head_waker(&self, waker: &Waker);
}
impl<T, B: Rb<T>> AsyncRbWrite<T> for AsyncRb<T, B> {
    fn register_head_waker(&self, waker: &Waker) {
        self.wakers.head.register(waker);
    }
}

impl<T, B: Rb<T>> Rb<T> for AsyncRb<T, B> {}

impl<T, B: Rb<T>> AsyncRb<T, B> {
    pub fn from_base(base: B) -> Self {
        Self {
            base,
            wakers: Wakers::default(),
            _phantom: PhantomData,
        }
    }

    pub fn into_base(self) -> B {
        self.base
    }

    pub fn split_ref(&mut self) -> (AsyncProducer<T, &Self>, AsyncConsumer<T, &Self>) {
        unsafe {
            (
                AsyncProducer::from_sync(Producer::new(self)),
                AsyncConsumer::from_sync(Consumer::new(self)),
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
                AsyncProducer::from_sync(Producer::new(rc.clone())),
                AsyncConsumer::from_sync(Consumer::new(rc)),
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
                AsyncProducer::from_sync(Producer::new(arc.clone())),
                AsyncConsumer::from_sync(Consumer::new(arc)),
            )
        }
    }
}

#[cfg(feature = "alloc")]
pub type AsyncHeapRb<T> = AsyncRb<T, HeapRb<T>>;

#[cfg(feature = "alloc")]
impl<T> AsyncHeapRb<T> {
    pub fn new(capacity: usize) -> Self {
        Self::from_base(HeapRb::new(capacity))
    }
}
