use super::{RbBase, RbRead, RbReadRef, RbWrite, RbWriteRef};
use core::{cell::Cell, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize, ptr};

pub struct RbReadCache<T, R: RbReadRef<T>> {
    target: R,
    head: Cell<usize>,
    tail: usize,
    _phantom: PhantomData<T>,
}

pub struct RbWriteCache<T, R: RbWriteRef<T>> {
    target: R,
    head: usize,
    tail: Cell<usize>,
    _phantom: PhantomData<T>,
}

impl<T, R: RbReadRef<T>> RbBase<T> for RbReadCache<T, R> {
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

impl<T, R: RbWriteRef<T>> RbBase<T> for RbWriteCache<T, R> {
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

impl<T, R: RbReadRef<T>> RbRead<T> for RbReadCache<T, R> {
    #[inline]
    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
}

impl<T, R: RbWriteRef<T>> RbWrite<T> for RbWriteCache<T, R> {
    #[inline]
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl<T, R: RbReadRef<T>> Drop for RbReadCache<T, R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: RbWriteRef<T>> Drop for RbWriteCache<T, R> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<T, R: RbReadRef<T>> RbReadCache<T, R> {
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
        unsafe { self.target.rb_read().set_head(self.head.get()) }
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

impl<T, R: RbWriteRef<T>> RbWriteCache<T, R> {
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
        unsafe { self.target.rb_write().set_tail(self.tail.get()) }
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
