use super::{Container, Rb, RbBase, RbRead, RbWrite, SharedStorage};
use crate::{consumer::Consumer, producer::Producer};
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbeam_utils::CachePadded;

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

/// Ring buffer that could be shared between threads.
///
/// Implements [`Sync`] *if `T` implements [`Send`]*. And therefore its [`Producer`] and [`Consumer`] implement [`Send`].
///
/// Note that there is no explicit requirement of `T: Send`. Instead [`SharedRb`] will work just fine even with `T: !Send`
/// until you try to send its [`Producer`] or [`Consumer`] to another thread.
#[cfg_attr(
    feature = "std",
    doc = r##"
```
use std::{thread, vec::Vec};
use ringbuf::SharedRb;

let (mut prod, mut cons) = SharedRb::<i32, Vec<_>>::new(256).split();
thread::spawn(move || {
    prod.push(123).unwrap();
})
.join();
thread::spawn(move || {
    assert_eq!(cons.pop().unwrap(), 123);
})
.join();
```
"##
)]
pub struct SharedRb<T, C: Container<T>> {
    storage: SharedStorage<T, C>,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl<T, C: Container<T>> RbBase<T> for SharedRb<T, C> {
    #[inline]
    unsafe fn slices(
        &self,
        head: usize,
        tail: usize,
    ) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        self.storage.as_mut_slices(head, tail)
    }

    #[inline]
    fn capacity_nonzero(&self) -> NonZeroUsize {
        self.storage.len()
    }

    #[inline]
    fn head(&self) -> usize {
        self.head.load(Ordering::Acquire)
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail.load(Ordering::Acquire)
    }
}

impl<T, C: Container<T>> RbRead<T> for SharedRb<T, C> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.store(value, Ordering::Release)
    }
}

impl<T, C: Container<T>> RbWrite<T> for SharedRb<T, C> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.store(value, Ordering::Release)
    }
}

impl<T, C: Container<T>> Rb<T> for SharedRb<T, C> {}

impl<T, C: Container<T>> Drop for SharedRb<T, C> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, C: Container<T>> SharedRb<T, C> {
    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see [`RbBase`](`crate::ring_buffer::RbBase`)).
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            storage: SharedStorage::new(container),
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    /// Destructures ring buffer into underlying container and `head` and `tail` counters.
    ///
    /// # Safety
    ///
    /// Initialized contents of the container must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (C, usize, usize) {
        let (head, tail) = (self.head(), self.tail());
        let self_ = ManuallyDrop::new(self);

        (ptr::read(&self_.storage).into_inner(), head, tail)
    }

    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in [`Arc`]. If you don't want to use heap the see [`Self::split_ref`].
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Producer<T, Arc<Self>>, Consumer<T, Arc<Self>>)
    where
        Self: Sized,
    {
        let arc = Arc::new(self);
        unsafe { (Producer::new(arc.clone()), Consumer::new(arc)) }
    }

    /// Splits ring buffer into producer and consumer without using the heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you also need to store the buffer somewhere.
    pub fn split_ref(&mut self) -> (Producer<T, &Self>, Consumer<T, &Self>)
    where
        Self: Sized,
    {
        unsafe { (Producer::new(self), Consumer::new(self)) }
    }
}
