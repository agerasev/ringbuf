use crate::{
    consumer::Consumer,
    observer::Observer,
    producer::Producer,
    raw::RawRb,
    ring_buffer::RingBuffer,
    storage::{Shared, Storage},
    stored::StoredRb,
};
use core::{
    mem::ManuallyDrop,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbeam_utils::CachePadded;

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
use std::{thread, sync::Arc};
use ringbuf::{SharedRb, storage::Heap, prelude::*};

let rb = SharedRb::<Heap<i32>>::new(256);
let (mut prod, mut cons) = Split::<Arc<_>>::split(rb);
thread::spawn(move || {
    prod.try_push(123).unwrap();
})
.join();
thread::spawn(move || {
    assert_eq!(cons.try_pop().unwrap(), 123);
})
.join();
```
"##
)]
pub struct SharedRb<S: Storage> {
    storage: Shared<S>,
    read: CachePadded<AtomicUsize>,
    write: CachePadded<AtomicUsize>,
}

impl<S: Storage> StoredRb for SharedRb<S> {
    type Storage = S;

    unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        Self {
            storage: Shared::new(storage),
            read: CachePadded::new(AtomicUsize::new(read)),
            write: CachePadded::new(AtomicUsize::new(write)),
        }
    }

    unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let (read, write) = (self.read_end(), self.write_end());
        let self_ = ManuallyDrop::new(self);
        (ptr::read(&self_.storage).into_inner(), read, write)
    }

    fn storage(&self) -> &Shared<S> {
        &self.storage
    }
}

impl<S: Storage> RawRb for SharedRb<S> {
    #[inline]
    fn read_end(&self) -> usize {
        self.read.load(Ordering::Acquire)
    }

    #[inline]
    fn write_end(&self) -> usize {
        self.write.load(Ordering::Acquire)
    }

    #[inline]
    unsafe fn set_read_end(&self, value: usize) {
        self.read.store(value, Ordering::Release)
    }

    #[inline]
    unsafe fn set_write_end(&self, value: usize) {
        self.write.store(value, Ordering::Release)
    }
}

impl<S: Storage> Observer for SharedRb<S> {
    type Item = S::Item;

    type Raw = Self;

    fn as_raw(&self) -> &Self::Raw {
        self
    }
}

impl<S: Storage> Producer for SharedRb<S> {}

impl<S: Storage> Consumer for SharedRb<S> {}

impl<S: Storage> RingBuffer for SharedRb<S> {}

impl<S: Storage> Drop for SharedRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}
