#[cfg(feature = "alloc")]
use super::traits::GenSplit;
use super::{macros::rb_impl_init, traits::GenSplitRef, utils::ranges};
#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    halves::{Cons, Prod},
    impl_consumer_traits, impl_producer_traits,
    storage::{Shared, Static, Storage},
    traits::{Consumer, Observer, Producer, RingBuffer},
};
#[cfg(feature = "alloc")]
use alloc::rc::Rc;
use core::{
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};

/// Ring buffer for single-threaded use only.
pub struct LocalRb<S: Storage> {
    storage: Shared<S>,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl<S: Storage> LocalRb<S> {
    /// Constructs ring buffer from storage and indices.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see [`RbBase`](`crate::ring_buffer::RbBase`)).
    pub unsafe fn from_raw_parts(storage: S, read: usize, write: usize) -> Self {
        Self {
            storage: Shared::new(storage),
            read: Cell::new(read),
            write: Cell::new(write),
        }
    }
    /// Destructures ring buffer into underlying storage and `read` and `write` indices.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, usize, usize) {
        let this = ManuallyDrop::new(self);
        (ptr::read(&this.storage).into_inner(), this.read.get(), this.write.get())
    }
}

impl<S: Storage> Observer for LocalRb<S> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.get()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }
}

impl<S: Storage> Producer for LocalRb<S> {
    #[inline]
    unsafe fn set_write_index(&mut self, value: usize) {
        self.unsafe_set_write_index(value);
    }
}

impl<S: Storage> Consumer for LocalRb<S> {
    #[inline]
    unsafe fn set_read_index(&mut self, value: usize) {
        self.unsafe_set_read_index(value);
    }
}

impl<S: Storage> RingBuffer for LocalRb<S> {
    #[inline]
    unsafe fn unsafe_set_write_index(&self, value: usize) {
        self.write.set(value);
    }
    #[inline]
    unsafe fn unsafe_set_read_index(&self, value: usize) {
        self.read.set(value);
    }
}

impl<S: Storage> Drop for LocalRb<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(feature = "alloc")]
unsafe impl<S: Storage, B: RingBuffer> GenSplit<B> for LocalRb<S> {
    type GenProd = Prod<Rc<B>>;
    type GenCons = Cons<Rc<B>>;

    fn gen_split(this: B) -> (Self::GenProd, Self::GenCons) {
        let rc = Rc::new(this);
        unsafe { (Prod::new(rc.clone()), Cons::new(rc)) }
    }
}
unsafe impl<'a, S: Storage + 'a, B: RingBuffer + 'a> GenSplitRef<'a, B> for LocalRb<S> {
    type GenRefProd = Prod<&'a B>;
    type GenRefCons = Cons<&'a B>;

    fn gen_split_ref(this: &'a mut B) -> (Self::GenRefProd, Self::GenRefCons) {
        unsafe { (Prod::new(this), Cons::new(this)) }
    }
}

rb_impl_init!(LocalRb);

impl_producer_traits!(LocalRb<S: Storage>);
impl_consumer_traits!(LocalRb<S: Storage>);
