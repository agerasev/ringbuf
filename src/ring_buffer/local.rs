use super::{Container, Rb, RbBase, RbRead, RbWrite, SharedStorage};
use crate::{consumer::Consumer, producer::Producer};
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

#[cfg(feature = "alloc")]
use alloc::rc::Rc;

/// Ring buffer for using in single thread.
///
/// Does *not* implement [`Sync`]. And its [`Producer`] and [`Consumer`] do *not* implement [`Send`].
///
#[cfg_attr(
    feature = "std",
    doc = r##"
This code must fail to compile:

```compile_fail
use std::{thread, vec::Vec};
use ringbuf::LocalRb;

let (mut prod, mut cons) = LocalRb::<i32, Vec<_>>::new(256).split();
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
pub struct LocalRb<T, C: Container<T>> {
    storage: SharedStorage<T, C>,
    head: Cell<usize>,
    tail: Cell<usize>,
}

impl<T, C: Container<T>> RbBase<T> for LocalRb<T, C> {
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
        self.head.get()
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail.get()
    }
}

impl<T, C: Container<T>> RbRead<T> for LocalRb<T, C> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
}

impl<T, C: Container<T>> RbWrite<T> for LocalRb<T, C> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl<T, C: Container<T>> Rb<T> for LocalRb<T, C> {}

impl<T, C: Container<T>> Drop for LocalRb<T, C> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, C: Container<T>> LocalRb<T, C> {
    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see [`RbBase`](`crate::ring_buffer::RbBase`)).
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            storage: SharedStorage::new(container),
            head: Cell::new(head),
            tail: Cell::new(tail),
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
    /// This method consumes the ring buffer and puts it on heap in [`Rc`]. If you don't want to use heap the see [`Self::split_ref`].
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Producer<T, Rc<Self>>, Consumer<T, Rc<Self>>)
    where
        Self: Sized,
    {
        let rc = Rc::new(self);
        unsafe { (Producer::new(rc.clone()), Consumer::new(rc)) }
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
