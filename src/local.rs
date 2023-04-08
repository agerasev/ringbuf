use crate::{
    consumer::Consumer,
    observer::Observer,
    producer::Producer,
    raw::RawRb,
    ring_buffer::RingBuffer,
    storage::{Shared, Storage},
    stored::{OwningRb, StoredRb},
};
use core::{cell::Cell, mem::ManuallyDrop, ptr};

/// Ring buffer for using in single thread.
///
/// Does *not* implement [`Sync`]. And its [`Producer`] and [`Consumer`] do *not* implement [`Send`].
///
#[cfg_attr(
    feature = "std",
    doc = r##"
This code must fail to compile:

```compile_fail
use std::{thread, sync::Arc};
use ringbuf::{LocalRb, storage::Static, prelude::*};

let rb = LocalRb::<Static<i32, 16>>::default();
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
pub struct LocalRb<S: Storage> {
    storage: Shared<S>,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<S: Storage> StoredRb for LocalRb<S> {
    type Storage = S;

    #[inline]
    fn storage(&self) -> &Shared<Self::Storage> {
        &self.storage
    }
}

impl<S: Storage> OwningRb for LocalRb<S> {
    unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        Self {
            storage: Shared::new(storage),
            read: Cell::new(read),
            write: Cell::new(write),
        }
    }
    unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let (read, write) = (self.read_end(), self.write_end());
        let self_ = ManuallyDrop::new(self);
        (ptr::read(&self_.storage).into_inner(), read, write)
    }
}

impl<S: Storage> RawRb for LocalRb<S> {
    #[inline]
    fn read_end(&self) -> usize {
        self.read.get()
    }

    #[inline]
    fn write_end(&self) -> usize {
        self.write.get()
    }

    #[inline]
    unsafe fn set_read_end(&self, value: usize) {
        self.read.set(value);
    }

    #[inline]
    unsafe fn set_write_end(&self, value: usize) {
        self.write.set(value);
    }
}

impl<S: Storage> Observer for LocalRb<S> {
    type Item = S::Item;

    type Raw = Self;

    fn as_raw(&self) -> &Self::Raw {
        self
    }
}

impl<S: Storage> Consumer for LocalRb<S> {}

impl<S: Storage> Producer for LocalRb<S> {}

impl<S: Storage> RingBuffer for LocalRb<S> {}

impl<S: Storage> Drop for LocalRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}
