#[cfg(feature = "alloc")]
use crate::alias::Arc;
#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
use crate::{sync::Semaphore, BlockingCons, BlockingProd};
use core::{mem::MaybeUninit, num::NonZeroUsize};
#[cfg(feature = "alloc")]
use ringbuf::traits::Split;
use ringbuf::{
    rb::RbRef,
    storage::Storage,
    traits::{Consumer, Observer, Producer, RingBuffer, SplitRef},
    SharedRb,
};

#[cfg(not(feature = "std"))]
pub struct BlockingRb<S: Storage, X: Semaphore> {
    base: SharedRb<S>,
    pub(crate) read: X,
    pub(crate) write: X,
}
#[cfg(feature = "std")]
pub struct BlockingRb<S: Storage, X: Semaphore = StdSemaphore> {
    base: SharedRb<S>,
    pub(crate) read: X,
    pub(crate) write: X,
}

impl<S: Storage, X: Semaphore> BlockingRb<S, X> {
    pub fn from(base: SharedRb<S>) -> Self {
        Self {
            base,
            read: X::default(),
            write: X::default(),
        }
    }
}

impl<S: Storage, X: Semaphore> Observer for BlockingRb<S, X> {
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

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<S::Item>], &[MaybeUninit<S::Item>]) {
        self.base.unsafe_slices(start, end)
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        self.base.unsafe_slices_mut(start, end)
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
impl<S: Storage, X: Semaphore> Producer for BlockingRb<S, X> {
    unsafe fn set_write_index(&self, value: usize) {
        self.base.set_write_index(value);
        self.write.give();
    }
}
impl<S: Storage, X: Semaphore> Consumer for BlockingRb<S, X> {
    unsafe fn set_read_index(&self, value: usize) {
        self.base.set_read_index(value);
        self.read.give();
    }
}
impl<S: Storage, X: Semaphore> RingBuffer for BlockingRb<S, X> {
    unsafe fn hold_read(&self, flag: bool) -> bool {
        let old = self.base.hold_read(flag);
        self.read.give();
        old
    }
    unsafe fn hold_write(&self, flag: bool) -> bool {
        let old = self.base.hold_write(flag);
        self.write.give();
        old
    }
}

impl<S: Storage, X: Semaphore> SplitRef for BlockingRb<S, X> {
    type RefProd<'a>
        = BlockingProd<&'a Self>
    where
        Self: 'a;
    type RefCons<'a>
        = BlockingCons<&'a Self>
    where
        Self: 'a;

    fn split_ref(&mut self) -> (Self::RefProd<'_>, Self::RefCons<'_>) {
        (BlockingProd::new(self), BlockingCons::new(self))
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage, X: Semaphore> Split for BlockingRb<S, X> {
    type Prod = BlockingProd<Arc<Self>>;
    type Cons = BlockingCons<Arc<Self>>;

    fn split(self) -> (Self::Prod, Self::Cons) {
        let arc = Arc::new(self);
        (BlockingProd::new(arc.clone()), BlockingCons::new(arc))
    }
}

pub trait BlockingRbRef: RbRef<Rb = BlockingRb<Self::Storage, Self::Semaphore>> {
    type Storage: Storage;
    type Semaphore: Semaphore;
}
impl<S: Storage, X: Semaphore, R: RbRef<Rb = BlockingRb<S, X>>> BlockingRbRef for R {
    type Storage = S;
    type Semaphore = X;
}

impl<S: Storage, X: Semaphore> AsRef<Self> for BlockingRb<S, X> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<S: Storage, X: Semaphore> AsMut<Self> for BlockingRb<S, X> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}
