use super::{
    macros::rb_impl_init,
    traits::{GenSplit, GenSplitRef},
    utils::ranges,
};
#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    halves::{CachedCons, CachedProd},
    impl_consumer_traits, impl_producer_traits,
    storage::{Shared, Static, Storage},
    traits::{Consumer, Observer, Producer, RingBuffer},
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

#[cfg(feature = "alloc")]
unsafe impl<S: Storage, B: RingBuffer> GenSplit<B> for SharedRb<S> {
    type GenProd = CachedProd<Arc<B>>;
    type GenCons = CachedCons<Arc<B>>;

    fn gen_split(this: B) -> (Self::GenProd, Self::GenCons) {
        let rc = Arc::new(this);
        unsafe { (CachedProd::new(rc.clone()), CachedCons::new(rc)) }
    }
}
unsafe impl<'a, S: Storage + 'a, B: RingBuffer + 'a> GenSplitRef<'a, B> for SharedRb<S> {
    type GenRefProd = CachedProd<&'a B>;
    type GenRefCons = CachedCons<&'a B>;

    fn gen_split_ref(this: &'a mut B) -> (Self::GenRefProd, Self::GenRefCons) {
        unsafe { (CachedProd::new(this), CachedCons::new(this)) }
    }
}

rb_impl_init!(SharedRb);

impl_producer_traits!(SharedRb<S: Storage>);
impl_consumer_traits!(SharedRb<S: Storage>);
