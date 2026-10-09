use futures_util::task::AtomicWaker;
use ringbuf::{
    Rb,
    indices::{AtomicIndices, Indices},
    markers::{AtomicMarkers, Markers, TrackedMarkers},
    rb::RbHandle,
    storage::Storage,
    traits::EndpointPolicy,
};
#[derive(Default)]
pub struct AsyncMarkers {
    flags: AtomicMarkers,
    pub(crate) read: AtomicWaker,
    pub(crate) write: AtomicWaker,
}
unsafe impl Markers for AsyncMarkers {
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
        self.read.wake();
    }
    fn notify_write(&self) {
        self.write.wake();
    }
}
impl TrackedMarkers for AsyncMarkers {}
pub type AsyncRb<S> = Rb<S, AtomicIndices, AsyncMarkers>;
pub trait AsyncRbHandle: RbHandle<Rb = Rb<Self::Storage, Self::Indices, AsyncMarkers>> {
    type Storage: Storage + ?Sized;
    type Indices: Indices;
}
impl<S: Storage + ?Sized, I: Indices, R: RbHandle<Rb = Rb<S, I, AsyncMarkers>>> AsyncRbHandle for R {
    type Storage = S;
    type Indices = I;
}
impl<S: Storage + ?Sized, I: Indices> EndpointPolicy<S, I> for AsyncMarkers {
    type Prod<R: RbHandle<Rb = Rb<S, I, Self>>> = crate::endpoint::AsyncProd<R>;
    type Cons<R: RbHandle<Rb = Rb<S, I, Self>>> = crate::endpoint::AsyncCons<R>;
    fn wrap_pair<R: RbHandle<Rb = Rb<S, I, Self>>>(
        pair: (ringbuf::endpoint::CachedProd<R>, ringbuf::endpoint::CachedCons<R>),
    ) -> (Self::Prod<R>, Self::Cons<R>) {
        (
            crate::endpoint::AsyncProd::from_cached(pair.0),
            crate::endpoint::AsyncCons::from_cached(pair.1),
        )
    }
}
pub use AsyncRbHandle as AsyncRbRef;
