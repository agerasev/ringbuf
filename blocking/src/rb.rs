use crate::sync::Semaphore;
#[cfg(feature = "std")]
use crate::sync::StdSemaphore;
use ringbuf::{
    Rb,
    indices::{AtomicIndices, Indices},
    markers::{AtomicMarkers, Markers, TrackedMarkers},
    rb::RbHandle,
    storage::Storage,
    traits::EndpointPolicy,
};
pub struct BlockingMarkers<X: Semaphore> {
    flags: AtomicMarkers,
    pub(crate) read: X,
    pub(crate) write: X,
}
impl<X: Semaphore> Default for BlockingMarkers<X> {
    fn default() -> Self {
        Self {
            flags: AtomicMarkers::default(),
            read: X::default(),
            write: X::default(),
        }
    }
}
unsafe impl<X: Semaphore> Markers for BlockingMarkers<X> {
    fn read_is_held(&self) -> bool {
        self.flags.read_is_held()
    }
    fn write_is_held(&self) -> bool {
        self.flags.write_is_held()
    }
    unsafe fn hold_read(&self, held: bool) -> bool {
        unsafe { self.flags.hold_read(held) }
    }
    unsafe fn hold_write(&self, held: bool) -> bool {
        unsafe { self.flags.hold_write(held) }
    }
    fn notify_read(&self) {
        self.read.give();
    }
    fn notify_write(&self) {
        self.write.give();
    }
}
impl<X: Semaphore> TrackedMarkers for BlockingMarkers<X> {}
#[cfg(feature = "std")]
pub type BlockingRb<S, X = StdSemaphore> = Rb<S, AtomicIndices, BlockingMarkers<X>>;
#[cfg(not(feature = "std"))]
pub type BlockingRb<S, X> = Rb<S, AtomicIndices, BlockingMarkers<X>>;
pub trait BlockingRbHandle: RbHandle<Rb = Rb<Self::Storage, Self::Indices, BlockingMarkers<Self::Semaphore>>> {
    type Storage: Storage + ?Sized;
    type Indices: Indices;
    type Semaphore: Semaphore;
}
impl<S: Storage + ?Sized, I: Indices, X: Semaphore, R: RbHandle<Rb = Rb<S, I, BlockingMarkers<X>>>> BlockingRbHandle for R {
    type Storage = S;
    type Indices = I;
    type Semaphore = X;
}
impl<S: Storage + ?Sized, I: Indices, X: Semaphore> EndpointPolicy<S, I> for BlockingMarkers<X> {
    type Prod<R: RbHandle<Rb = Rb<S, I, Self>>> = crate::endpoint::BlockingProd<R>;
    type Cons<R: RbHandle<Rb = Rb<S, I, Self>>> = crate::endpoint::BlockingCons<R>;
    fn wrap_pair<R: RbHandle<Rb = Rb<S, I, Self>>>(
        pair: (ringbuf::endpoint::CachedProd<R>, ringbuf::endpoint::CachedCons<R>),
    ) -> (Self::Prod<R>, Self::Cons<R>) {
        (
            crate::endpoint::BlockingProd::from_cached(pair.0),
            crate::endpoint::BlockingCons::from_cached(pair.1),
        )
    }
}
pub use BlockingRbHandle as BlockingRbRef;
