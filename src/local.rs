use crate::storage::StoredRb;

use super::{
    raw::{ranges, RawConsumer, RawProducer, RawRb},
    storage::{SharedStorage, Storage},
    Consumer, Observer, Producer, RingBuffer,
};
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

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
pub struct LocalRb<S: Storage> {
    storage: SharedStorage<S>,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<S: Storage> RawRb for LocalRb<S> {
    type Item = S::Item;

    #[inline]
    unsafe fn slices(
        &self,
        read: usize,
        write: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        let (first, second) = ranges(<Self as RawRb>::capacity(self), read, write);
        (
            self.storage.index_mut(first),
            self.storage.index_mut(second),
        )
    }

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    #[inline]
    fn read_end(&self) -> usize {
        self.read.get()
    }

    #[inline]
    fn write_end(&self) -> usize {
        self.write.get()
    }
}

impl<S: Storage> RawConsumer for LocalRb<S> {
    #[inline]
    unsafe fn set_read_end(&self, value: usize) {
        self.read.set(value);
    }
}

impl<S: Storage> RawProducer for LocalRb<S> {
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

impl<S: Storage> StoredRb for LocalRb<S> {
    type Storage = S;

    unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        Self {
            storage: SharedStorage::new(storage),
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
