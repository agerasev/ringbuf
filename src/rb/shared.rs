use super::{iter::PopIter, macros::rb_impl_init, traits::RbRef, utils::ranges};
#[cfg(feature = "alloc")]
use crate::traits::Split;
use crate::{
    halves::{CachingCons, CachingProd},
    impl_consumer_traits, impl_producer_traits,
    storage::{Shared, Static, Storage},
    traits::{Consumer, Observer, Producer, RingBuffer, SplitRef},
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

    type IntoIter = PopIter<Self>;
    fn into_iter(self) -> Self::IntoIter {
        unsafe { PopIter::new(self) }
    }

    type PopIter<'a> = PopIter<&'a Self> where S: 'a;
    fn pop_iter(&mut self) -> Self::PopIter<'_> {
        unsafe { PopIter::new(self) }
    }
}

impl<S: Storage> RingBuffer for SharedRb<S> {}

impl<S: Storage> Drop for SharedRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<S: Storage> AsRef<SharedRb<S>> for SharedRb<S> {
    fn as_ref(&self) -> &SharedRb<S> {
        self
    }
}
unsafe impl<S: Storage> RbRef for SharedRb<S> {
    type Target = SharedRb<S>;
}

#[cfg(feature = "alloc")]
impl<S: Storage> Split for SharedRb<S> {
    type Prod = CachingProd<Arc<Self>>;
    type Cons = CachingCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let rc = Arc::new(self);
        unsafe { (CachingProd::new(rc.clone()), CachingCons::new(rc)) }
    }
}
impl<S: Storage> SplitRef for SharedRb<S> {
    type RefProd<'a> = CachingProd<&'a Self> where Self: 'a;
    type RefCons<'a> = CachingCons<&'a Self> where Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        unsafe { (CachingProd::new(self), CachingCons::new(self)) }
    }
}

rb_impl_init!(SharedRb);

impl_producer_traits!(SharedRb<S: Storage>);
impl_consumer_traits!(SharedRb<S: Storage>);
