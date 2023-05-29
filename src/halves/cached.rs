use crate::{
    //cached::CachedCons,
    delegate_observer_methods,
    traits::{Consumer, Observer, Producer, RingBuffer},
};
use core::{cell::Cell, mem::MaybeUninit, ops::Deref};

/// Producer wrapper of ring buffer.
pub struct CachedProd<R: Deref>
where
    R::Target: RingBuffer,
{
    base: R,
    read: Cell<usize>,
}

/// Consumer wrapper of ring buffer.
pub struct CachedCons<R: Deref>
where
    R::Target: RingBuffer,
{
    base: R,
    write: Cell<usize>,
}

impl<R: Deref> CachedProd<R>
where
    R::Target: RingBuffer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(base: R) -> Self {
        Self {
            read: Cell::new(base.read_index()),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }
    pub fn into_base(self) -> R {
        self.base
    }

    pub fn fetch(&self) {
        self.read.set(self.base.read_index());
    }
}

impl<R: Deref> CachedCons<R>
where
    R::Target: RingBuffer,
{
    /// # Safety
    ///
    /// There must be no more than one consumer wrapper.
    pub unsafe fn new(base: R) -> Self {
        Self {
            write: Cell::new(base.write_index()),
            base,
        }
    }
    pub fn base(&self) -> &R {
        &self.base
    }
    pub fn into_base(self) -> R {
        self.base
    }

    pub fn fetch(&self) {
        self.write.set(self.base.write_index());
    }
}

impl<R: Deref> Observer for CachedProd<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Deref> Observer for CachedCons<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Deref> Producer for CachedProd<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.base.set_write_index(value);
    }

    #[inline]
    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        self.fetch();
        let (first, second) = unsafe { rb.unsafe_slices(rb.write_index(), self.read.get() + rb.capacity().get()) };
        (first as &_, second as &_)
    }
    #[inline]
    fn vacant_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        self.fetch();
        unsafe { rb.unsafe_slices(rb.write_index(), self.read.get() + rb.capacity().get()) }
    }
}

impl<R: Deref> Consumer for CachedCons<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.base.set_read_index(value)
    }

    #[inline]
    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        self.fetch();
        let (first, second) = unsafe { rb.unsafe_slices(rb.read_index(), self.write.get()) };
        (first as &_, second as &_)
    }
    #[inline]
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        self.fetch();
        rb.unsafe_slices(rb.read_index(), self.write.get())
    }
}

macro_rules! impl_prod_traits {
    ($CachedProd:ident) => {
        #[cfg(feature = "std")]
        impl<R: core::ops::Deref> std::io::Write for $CachedProd<R>
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

        impl<R: core::ops::Deref> core::fmt::Write for $CachedProd<R>
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

impl_prod_traits!(CachedProd);
/*
impl<R: Deref> CachedProd<R>
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

macro_rules! impl_cons_traits {
    ($CachedCons:ident) => {
        impl<R: core::ops::Deref> IntoIterator for $CachedCons<R>
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
        impl<R: core::ops::Deref> std::io::Read for $CachedCons<R>
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

impl_cons_traits!(CachedCons);

/*
impl<R: Deref> CachedCons<R>
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
