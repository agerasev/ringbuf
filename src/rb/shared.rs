use super::{macros::rb_impl_init, utils::ranges};
#[cfg(feature = "alloc")]
use crate::traits::Split;
use crate::{
    storage::{Shared, Static, Storage},
    traits::{
        consumer::{impl_consumer_traits, Consumer},
        producer::{impl_producer_traits, Producer},
        Observer, RingBuffer, SplitRef,
    },
    wrap::{CachingCons, CachingProd},
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use crossbeam_utils::CachePadded;

/// Ring buffer that can be shared between threads.
///
/// Note that there is no explicit requirement of `T: Send`. Instead ring buffer will work just fine even with `T: !Send`
/// until you try to send its producer or consumer to another thread.
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
    read_index: CachePadded<AtomicUsize>,
    write_index: CachePadded<AtomicUsize>,
    read_held: AtomicBool,
    write_held: AtomicBool,
}

impl<S: Storage> SharedRb<S> {
    /// Constructs ring buffer from storage and indices.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see implementation details).
    pub unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        Self {
            storage: Shared::new(storage),
            read_index: CachePadded::new(AtomicUsize::new(read)),
            write_index: CachePadded::new(AtomicUsize::new(write)),
            read_held: AtomicBool::new(false),
            write_held: AtomicBool::new(false),
        }
    }
    /// Destructures ring buffer into underlying storage and `read` and `write` indices.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let this = ManuallyDrop::new(self);
        (ptr::read(&this.storage).into_inner(), this.read_index(), this.write_index())
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
        self.read_index.load(Ordering::Acquire)
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write_index.load(Ordering::Acquire)
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }

    #[inline]
    fn read_is_held(&self) -> bool {
        self.read_held.load(Ordering::Relaxed)
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.write_held.load(Ordering::Relaxed)
    }
}

impl<S: Storage> Producer for SharedRb<S> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write_index.store(value, Ordering::Release);
    }
}

impl<S: Storage> Consumer for SharedRb<S> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read_index.store(value, Ordering::Release);
    }
}

impl<S: Storage> RingBuffer for SharedRb<S> {
    #[inline]
    unsafe fn hold_read(&self, flag: bool) -> bool {
        self.read_held.swap(flag, Ordering::Relaxed)
    }
    #[inline]
    unsafe fn hold_write(&self, flag: bool) -> bool {
        self.write_held.swap(flag, Ordering::Relaxed)
    }
}

impl<S: Storage> Drop for SharedRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(feature = "alloc")]
impl<S: Storage> Split for SharedRb<S> {
    type Prod = CachingProd<Arc<Self>>;
    type Cons = CachingCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let rc = Arc::new(self);
        (CachingProd::new(rc.clone()), CachingCons::new(rc))
    }
}
impl<S: Storage> SplitRef for SharedRb<S> {
    type RefProd<'a> = CachingProd<&'a Self> where Self: 'a;
    type RefCons<'a> = CachingCons<&'a Self> where Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        (CachingProd::new(self), CachingCons::new(self))
    }
}

rb_impl_init!(SharedRb);

impl_producer_traits!(SharedRb<S: Storage>);
impl_consumer_traits!(SharedRb<S: Storage>);

impl<S: Storage> AsRef<Self> for SharedRb<S> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<S: Storage> AsMut<Self> for SharedRb<S> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}
