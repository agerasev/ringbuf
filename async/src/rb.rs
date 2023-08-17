use crate::wrap::{AsyncCons, AsyncProd};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use core::{mem::MaybeUninit, num::NonZeroUsize};
use futures::task::AtomicWaker;
#[cfg(feature = "alloc")]
use ringbuf::traits::Split;
use ringbuf::{
    rb::traits::RbRef,
    storage::Storage,
    traits::{Consumer, Observer, Producer, RingBuffer, SplitRef},
    SharedRb,
};

pub trait AsyncRbRef: RbRef<Target = AsyncRb<Self::Storage>> {
    type Storage: Storage;
}
impl<S: Storage, R: RbRef<Target = AsyncRb<S>>> AsyncRbRef for R {
    type Storage = S;
}

pub struct AsyncRb<S: Storage> {
    base: SharedRb<S>,
    pub(crate) read: AtomicWaker,
    pub(crate) write: AtomicWaker,
}

impl<S: Storage> AsyncRb<S> {
    pub fn from(base: SharedRb<S>) -> Self {
        Self {
            base,
            read: AtomicWaker::default(),
            write: AtomicWaker::default(),
        }
    }
}

impl<S: Storage> Unpin for AsyncRb<S> {}

impl<S: Storage> Observer for AsyncRb<S> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.base.read_index()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.base.write_index()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        self.base.unsafe_slices(start, end)
    }

    #[inline]
    fn read_is_held(&self) -> bool {
        self.base.read_is_held()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.base.write_is_held()
    }
}

impl<S: Storage> Producer for AsyncRb<S> {
    unsafe fn set_write_index(&self, value: usize) {
        self.base.set_write_index(value);
        self.write.wake();
    }
    fn close(&mut self) {}
}
impl<S: Storage> Consumer for AsyncRb<S> {
    unsafe fn set_read_index(&self, value: usize) {
        self.base.set_read_index(value);
        self.read.wake();
    }
    fn close(&mut self) {}
}
impl<S: Storage> RingBuffer for AsyncRb<S> {
    #[inline]
    fn hold_read(&self, flag: bool) {
        self.base.hold_read(flag);
        self.read.wake()
    }
    #[inline]
    fn hold_write(&self, flag: bool) {
        self.base.hold_write(flag);
        self.write.wake()
    }
}

impl<S: Storage> SplitRef for AsyncRb<S> {
    type RefProd<'a> = AsyncProd<&'a Self> where Self:  'a;
    type RefCons<'a> = AsyncCons<&'a Self> where Self:  'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        unsafe { (AsyncProd::new(self), AsyncCons::new(self)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage> Split for AsyncRb<S> {
    type Prod = AsyncProd<Arc<Self>>;
    type Cons = AsyncCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let arc = Arc::new(self);
        unsafe { (AsyncProd::new(arc.clone()), AsyncCons::new(arc)) }
    }
}
