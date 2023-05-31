use super::{macros::rb_impl_init, utils::ranges};
#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    consumer::Consumer,
    halves::cached::{CachedCons, CachedProd},
    producer::Producer,
    storage::{Shared, Static, Storage},
    traits::{ring_buffer::Split, Observer, RingBuffer},
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

/// Ring buffer that can be shared between threads.
///
/// Note that there is no explicit requirement of `T: Send`. Instead [`Rb`] will work just fine even with `T: !Send`
/// until you try to send its [`Prod`] or [`Cons`] to another thread.
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
    /// Constructs ring buffer from storage and indices.
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
    /// Destructures ring buffer into underlying storage and `read` and `write` indices.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let this = ManuallyDrop::new(self);
        (
            ptr::read(&this.storage).into_inner(),
            this.read.load(Ordering::Acquire),
            this.write.load(Ordering::Acquire),
        )
    }
}

impl<S: Storage> Observer for SharedRb<S> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.load(Ordering::Acquire)
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.load(Ordering::Acquire)
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }
}

impl<S: Storage> Producer for SharedRb<S> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.store(value, Ordering::Release);
    }
}

impl<S: Storage> Consumer for SharedRb<S> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.store(value, Ordering::Release);
    }
}

impl<S: Storage> RingBuffer for SharedRb<S> {}

impl<S: Storage> Drop for SharedRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<'a, S: Storage + 'a> Split for &'a mut SharedRb<S> {
    type Prod = CachedProd<&'a SharedRb<S>>;
    type Cons = CachedCons<&'a SharedRb<S>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        unsafe { (CachedProd::new(self), CachedCons::new(self)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage> Split for SharedRb<S> {
    type Prod = CachedProd<Arc<Self>>;
    type Cons = CachedCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let rc = Arc::new(self);
        unsafe { (CachedProd::new(rc.clone()), CachedCons::new(rc)) }
    }
}
impl<S: Storage> SharedRb<S> {
    #[cfg(feature = "alloc")]
    pub fn split(self) -> (CachedProd<Arc<Self>>, CachedCons<Arc<Self>>) {
        Split::split(self)
    }
    pub fn split_ref(&mut self) -> (CachedProd<&Self>, CachedCons<&Self>) {
        Split::split(self)
    }
}

rb_impl_init!(SharedRb);
