use crate::{
    consumer::{Cons, Consumer},
    producer::Prod,
    raw::{ranges, AsRaw, RawRb, RbMarker},
    storage::{impl_rb_ctors, Shared, Storage},
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
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
use std::thread;
use ringbuf::{SharedRb, storage::Heap, traits::*};

let rb = SharedRb::<Heap<i32>>::new(256);
let (mut prod, mut cons) = rb.split();
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

impl<S: Storage> SharedRb<S> {
    /// Constructs ring buffer from storage and counters.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see [`RbBase`](`crate::ring_buffer::RbBase`)).
    pub unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        Self {
            storage: Shared::new(storage),
            read: CachePadded::new(AtomicUsize::new(read)),
            write: CachePadded::new(AtomicUsize::new(write)),
        }
    }
    /// Destructures ring buffer into underlying storage and `read` and `write` counters.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let (read, write) = (self.read_index(), self.write_index());
        let self_ = ManuallyDrop::new(self);
        (ptr::read(&self_.storage).into_inner(), read, write)
    }

    pub fn split_ref(&mut self) -> (Prod<&Self>, Cons<&Self>) {
        unsafe { (Prod::new(self), Cons::new(self)) }
    }
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (Prod<Arc<Self>>, Cons<Arc<Self>>) {
        let arc = Arc::new(self);
        unsafe { (Prod::new(arc.clone()), Cons::new(arc)) }
    }
}

impl<S: Storage> RawRb for SharedRb<S> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    unsafe fn slices(
        &self,
        start: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.load(Ordering::Acquire)
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.load(Ordering::Acquire)
    }

    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.store(value, Ordering::Release)
    }

    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.store(value, Ordering::Release)
    }
}

impl<S: Storage> AsRaw for SharedRb<S> {
    type Raw = Self;
    #[inline]
    fn as_raw(&self) -> &Self::Raw {
        self
    }
}
unsafe impl<S: Storage> RbMarker for SharedRb<S> {}

impl<S: Storage> Drop for SharedRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl_rb_ctors!(SharedRb);
