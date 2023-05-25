use crate::{
    //cached::CachedProd,
    delegate_observer_methods,
    traits::{Observer, Producer, RingBuffer},
};
use core::{mem::MaybeUninit, ops::Deref};

/// Producer wrapper of ring buffer.
pub struct Prod<R: Deref>
where
    R::Target: RingBuffer,
{
    base: R,
}

impl<R: Deref> Prod<R>
where
    R::Target: RingBuffer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(base: R) -> Self {
        Self { base }
    }
    pub fn base(&self) -> &R {
        &self.base
    }
    pub fn into_base(self) -> R {
        self.base
    }
}

impl<R: Deref> Observer for Prod<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Deref> Producer for Prod<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn advance_write_index(&self, count: usize) {
        self.base.advance_write_index(count);
    }

    #[inline]
    unsafe fn unsafe_vacant_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.unsafe_vacant_slices()
    }
}

macro_rules! impl_prod_traits {
    ($Prod:ident) => {
        #[cfg(feature = "std")]
        impl<R: core::ops::Deref> std::io::Write for $Prod<R>
        where
            R::Target: crate::traits::RingBuffer<Item = u8>,
        {
            fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
                use crate::producer::Producer;
                let n = self.push_slice(buffer);
                if n == 0 && !buffer.is_empty() {
                    Err(std::io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(n)
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<R: core::ops::Deref> core::fmt::Write for $Prod<R>
        where
            R::Target: crate::traits::RingBuffer<Item = u8>,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                use crate::producer::Producer;
                let n = self.push_slice(s.as_bytes());
                if n != s.len() {
                    Err(core::fmt::Error::default())
                } else {
                    Ok(())
                }
            }
        }
    };
}
pub(crate) use impl_prod_traits;

impl_prod_traits!(Prod);
/*
impl<R: Deref> Prod<R>
where
    R::Target: RingBuffer,
{
    pub fn cached(&mut self) -> CachedProd<&R::Target> {
        unsafe { CachedProd::new(&self.base) }
    }
    pub fn into_cached(self) -> CachedProd<R> {
        unsafe { CachedProd::new(self.base) }
    }
}
*/
