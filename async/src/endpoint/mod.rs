mod cons;
mod prod;

use crate::rb::AsyncRbHandle;
use ringbuf::{
    Obs,
    endpoint::{Endpoint, direct::Direct},
    traits::{Delegate, observer::DelegateObserver},
};

pub struct AsyncEndpoint<R: AsyncRbHandle, const P: bool, const C: bool> {
    base: Direct<R, P, C>,
    pub(crate) stream_done: bool,
}

pub type AsyncProd<R> = AsyncEndpoint<R, true, false>;
pub type AsyncCons<R> = AsyncEndpoint<R, false, true>;

impl<R: AsyncRbHandle, const P: bool, const C: bool> AsyncEndpoint<R, P, C> {
    pub fn new(rb: R) -> Self {
        Self {
            base: Direct::new(rb),
            stream_done: false,
        }
    }

    pub fn from_cached(base: ringbuf::endpoint::Cached<R, P, C>) -> Self {
        Self {
            base: base.into_direct(),
            stream_done: false,
        }
    }
    pub fn try_new(rb: R) -> Result<Self, (ringbuf::endpoint::AcquireError, R)> {
        Direct::try_new(rb).map(|base| Self { base, stream_done: false })
    }
    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}

impl<R: AsyncRbHandle, const P: bool, const C: bool> Delegate for AsyncEndpoint<R, P, C> {
    type Base = Direct<R, P, C>;
    fn base(&self) -> &Self::Base {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        &mut self.base
    }
}

unsafe impl<R: AsyncRbHandle, const P: bool, const C: bool> Endpoint for AsyncEndpoint<R, P, C> {
    type Handle = R;
    fn rb_handle(&self) -> &R {
        self.base().rb_handle()
    }
    fn into_rb_handle(self) -> R {
        self.base.into_rb_handle()
    }
}

impl<R: AsyncRbHandle, const P: bool, const C: bool> Unpin for AsyncEndpoint<R, P, C> {}

unsafe impl<R: AsyncRbHandle, const P: bool, const C: bool> DelegateObserver for AsyncEndpoint<R, P, C> {}

impl<R: AsyncRbHandle, const P: bool, const C: bool> AsRef<Self> for AsyncEndpoint<R, P, C> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<R: AsyncRbHandle, const P: bool, const C: bool> AsMut<Self> for AsyncEndpoint<R, P, C> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<R: AsyncRbHandle> AsyncEndpoint<R, true, false> {
    pub fn defer(&mut self) -> ringbuf::endpoint::DeferredProd<&mut ringbuf::endpoint::direct::Direct<R, true, false>> {
        self.base.defer()
    }
    pub fn into_deferred(self) -> ringbuf::endpoint::DeferredProd<ringbuf::endpoint::direct::Direct<R, true, false>> {
        self.base.into_deferred()
    }
}

impl<R: AsyncRbHandle> AsyncEndpoint<R, false, true> {
    pub fn defer(&mut self) -> ringbuf::endpoint::DeferredCons<&mut ringbuf::endpoint::direct::Direct<R, false, true>> {
        self.base.defer()
    }
    pub fn into_deferred(self) -> ringbuf::endpoint::DeferredCons<ringbuf::endpoint::direct::Direct<R, false, true>> {
        self.base.into_deferred()
    }
}
