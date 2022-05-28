use super::{RbBase, RbRead, RbRef, RbWrite};
use core::{cell::Cell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ptr};

pub struct RbReadCache<T, R: RbRef<T>>
where
    R::Rb: RbRead<T>,
{
    target: R,
    head: Cell<usize>,
    tail: usize,
    _phantom: PhantomData<T>,
}

pub struct RbWriteCache<T, R: RbRef<T>>
where
    R::Rb: RbWrite<T>,
{
    target: R,
    head: usize,
    tail: Cell<usize>,
    _phantom: PhantomData<T>,
}

impl<T, R: RbRef<T>> RbBase<T> for RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.target.rb().data()
    }

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.target.rb().capacity()
    }

    #[inline]
    fn head(&self) -> usize {
        self.head.get()
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail
    }
}

impl<T, R: RbRef<T>> RbBase<T> for RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    #[inline]
    unsafe fn data(&self) -> &mut [MaybeUninit<T>] {
        self.target.rb().data()
    }

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.target.rb().capacity()
    }

    #[inline]
    fn head(&self) -> usize {
        self.head
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail.get()
    }
}

impl<T, R: RbRef<T>> RbRead<T> for RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
}

impl<T, R: RbRef<T>> RbWrite<T> for RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl<T, R: RbRef<T>> Drop for RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: RbRef<T>> Drop for RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: RbRef<T>> RbReadCache<T, R>
where
    R::Rb: RbRead<T>,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(rb_ref: R) -> Self {
        Self {
            head: Cell::new(rb_ref.rb().head()),
            tail: rb_ref.rb().tail(),
            target: rb_ref,
            _phantom: PhantomData,
        }
    }

    pub fn commit(&mut self) {
        unsafe { self.target.rb().set_head(self.head.get()) }
    }

    pub fn sync(&mut self) {
        self.commit();
        self.tail = self.target.rb().tail();
    }

    pub fn release(mut self) -> R {
        self.commit();
        let self_uninit = MaybeUninit::new(self);
        unsafe { ptr::read(&self_uninit.assume_init_ref().target) }
        // Self::drop is not called.
    }
}

impl<T, R: RbRef<T>> RbWriteCache<T, R>
where
    R::Rb: RbWrite<T>,
{
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(rb_ref: R) -> Self {
        Self {
            head: rb_ref.rb().head(),
            tail: Cell::new(rb_ref.rb().tail()),
            target: rb_ref,
            _phantom: PhantomData,
        }
    }

    pub fn commit(&mut self) {
        unsafe { self.target.rb().set_tail(self.tail.get()) }
    }

    pub fn sync(&mut self) {
        self.commit();
        self.head = self.target.rb().head();
    }

    pub fn release(mut self) -> R {
        self.commit();
        let self_uninit = MaybeUninit::new(self);
        unsafe { ptr::read(&self_uninit.assume_init_ref().target) }
        // Self::drop is not called.
    }
}
