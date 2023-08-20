use crate::{
    rb::BlockingRbRef,
    sync::Semaphore,
    traits::{BlockingConsumer, BlockingProducer},
};
use core::time::Duration;
use ringbuf::{
    rb::traits::ToRbRef,
    traits::{
        consumer::DelegateConsumer,
        observer::{DelegateObserver, Observer},
        producer::DelegateProducer,
        Based,
    },
    wrap::caching::Caching,
    Obs,
};
#[cfg(feature = "std")]
use std::io;

pub struct BlockingWrap<R: BlockingRbRef, const P: bool, const C: bool> {
    base: Option<Caching<R, P, C>>,
    timeout: Option<Duration>,
}
pub type BlockingProd<R> = BlockingWrap<R, true, false>;
pub type BlockingCons<R> = BlockingWrap<R, false, true>;

impl<R: BlockingRbRef, const P: bool, const C: bool> BlockingWrap<R, P, C> {
    pub fn new(rb: R) -> Self {
        Self {
            base: Some(Caching::new(rb)),
            timeout: None,
        }
    }

    pub fn observe(&self) -> Obs<R> {
        self.base().observe()
    }
}
impl<R: BlockingRbRef, const P: bool, const C: bool> Based for BlockingWrap<R, P, C> {
    type Base = Caching<R, P, C>;
    fn base(&self) -> &Self::Base {
        self.base.as_ref().unwrap()
    }
    fn base_mut(&mut self) -> &mut Self::Base {
        self.base.as_mut().unwrap()
    }
}
impl<R: BlockingRbRef, const P: bool, const C: bool> ToRbRef for BlockingWrap<R, P, C> {
    type RbRef = R;
    fn rb_ref(&self) -> &Self::RbRef {
        self.base().rb_ref()
    }
    fn into_rb_ref(self) -> Self::RbRef {
        self.base.unwrap().into_rb_ref()
    }
}

impl<R: BlockingRbRef> DelegateObserver for BlockingProd<R> {}
impl<R: BlockingRbRef> DelegateProducer for BlockingProd<R> {}

impl<R: BlockingRbRef> BlockingProducer for BlockingProd<R> {
    type Instant = <R::Semaphore as Semaphore>::Instant;

    fn wait_vacant(&mut self, count: usize, timeout: Option<Duration>) -> bool {
        let rb = self.rb();
        debug_assert!(count <= rb.capacity().get());
        rb.read.try_take();
        if rb.vacant_len() >= count {
            return true;
        }
        if rb.read.take(timeout) {
            rb.vacant_len() >= count
        } else {
            false
        }
    }

    fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

impl<R: BlockingRbRef> DelegateObserver for BlockingCons<R> {}
impl<R: BlockingRbRef> DelegateConsumer for BlockingCons<R> {}

impl<R: BlockingRbRef> BlockingConsumer for BlockingCons<R> {
    type Instant = <R::Semaphore as Semaphore>::Instant;
    fn wait_occupied(&mut self, count: usize, timeout: Option<Duration>) -> bool {
        let rb = self.rb();
        debug_assert!(count <= rb.capacity().get());
        rb.write.try_take();
        if rb.occupied_len() >= count {
            return true;
        }
        if rb.write.take(timeout) {
            rb.occupied_len() >= count
        } else {
            false
        }
    }

    fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

#[cfg(feature = "std")]
impl<R: BlockingRbRef> io::Write for BlockingProd<R>
where
    Self: BlockingProducer<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        <Self as BlockingProducer>::write(self, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<R: BlockingRbRef> io::Read for BlockingCons<R>
where
    Self: BlockingConsumer<Item = u8>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        <Self as BlockingConsumer>::read(self, buf)
    }
}
