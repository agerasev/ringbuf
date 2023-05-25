use crate::{
    //cached::CachedCons,
    delegate_observer_methods,
    traits::{Consumer, Observer, RingBuffer},
};
use core::{mem::MaybeUninit, ops::Deref};

/// Consumer wrapper of ring buffer.
pub struct Cons<R: Deref>
where
    R::Target: RingBuffer,
{
    base: R,
}

impl<R: Deref> Cons<R>
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

impl<R: Deref> Observer for Cons<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    delegate_observer_methods!(Self::base);
}
impl<R: Deref> Consumer for Cons<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn advance_read_index(&self, count: usize) {
        self.base.advance_read_index(count)
    }

    #[inline]
    unsafe fn unsafe_occupied_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        self.base.unsafe_occupied_slices()
    }
}

macro_rules! impl_cons_traits {
    ($Cons:ident) => {
        impl<R: core::ops::Deref> IntoIterator for $Cons<R>
        where
            R::Target: RingBuffer,
        {
            type Item = <R::Target as crate::traits::Observer>::Item;
            type IntoIter = crate::consumer::IntoIter<Self>;

            fn into_iter(self) -> Self::IntoIter {
                crate::consumer::IntoIter(self)
            }
        }

        #[cfg(feature = "std")]
        impl<R: core::ops::Deref> std::io::Read for $Cons<R>
        where
            R::Target: crate::traits::RingBuffer<Item = u8>,
        {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                use crate::consumer::Consumer;
                let n = self.pop_slice(buffer);
                if n == 0 && !buffer.is_empty() {
                    Err(std::io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(n)
                }
            }
        }
    };
}
pub(crate) use impl_cons_traits;

impl_cons_traits!(Cons);

/*
impl<R: Deref> Cons<R>
where
    R::Target: RingBuffer,
{
    pub fn cached(&mut self) -> CachedCons<&R::Target> {
        unsafe { CachedCons::new(&self.base) }
    }
    pub fn into_cached(self) -> CachedCons<R> {
        unsafe { CachedCons::new(self.base) }
    }
}
*/
