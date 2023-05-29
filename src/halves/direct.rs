use super::macros::{impl_cons_traits, impl_prod_traits};
use crate::{
    //cached::FrozenCons,
    delegate_observer_methods,
    frozen::{FrozenCons, FrozenProd},
    traits::{Consumer, Observer, Producer, RingBuffer},
};
use core::{mem::MaybeUninit, ops::Deref};

/// Producer wrapper of ring buffer.
pub struct Prod<R: Deref>
where
    R::Target: RingBuffer,
{
    base: R,
}

/// Consumer wrapper of ring buffer.
pub struct Cons<R: Deref>
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

impl<R: Deref> Observer for Prod<R>
where
    R::Target: RingBuffer,
{
    type Item = <R::Target as Observer>::Item;

    delegate_observer_methods!(Self::base);
}

impl<R: Deref> Observer for Cons<R>
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
    unsafe fn set_write_index(&self, value: usize) {
        self.base.set_write_index(value)
    }

    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        let (first, second) = unsafe { rb.unsafe_slices(rb.write_index(), rb.read_index() + rb.capacity().get()) };
        (first as &_, second as &_)
    }
    fn vacant_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        unsafe { rb.unsafe_slices(rb.write_index(), rb.read_index() + rb.capacity().get()) }
    }
}

impl<R: Deref> Consumer for Cons<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.base.set_read_index(value)
    }

    fn occupied_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        let (first, second) = unsafe { rb.unsafe_slices(rb.read_index(), rb.write_index()) };
        (first as &_, second as &_)
    }
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        let rb = self.base.deref();
        rb.unsafe_slices(rb.read_index(), rb.write_index())
    }
}

impl_prod_traits!(Prod);
impl_cons_traits!(Cons);

impl<R: Deref> Prod<R>
where
    R::Target: RingBuffer,
{
    pub fn freeze(&mut self) -> FrozenProd<&R::Target> {
        unsafe { FrozenProd::new(&self.base) }
    }
    pub fn into_frozen(self) -> FrozenProd<R> {
        unsafe { FrozenProd::new(self.base) }
    }
}

impl<R: Deref> Cons<R>
where
    R::Target: RingBuffer,
{
    pub fn freeze(&mut self) -> FrozenCons<&R::Target> {
        unsafe { FrozenCons::new(&self.base) }
    }
    pub fn into_frozen(self) -> FrozenCons<R> {
        unsafe { FrozenCons::new(self.base) }
    }
}
