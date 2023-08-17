use super::direct::Obs;
use crate::{
    rb::traits::{RbRef, ToRbRef},
    traits::{Consumer, Observer, Producer},
};
use core::{
    cell::Cell,
    fmt,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr,
};
#[cfg(feature = "std")]
use std::io;

pub struct Frozen<R: RbRef, const P: bool, const C: bool> {
    rb: R,
    read: Cell<usize>,
    write: Cell<usize>,
}

/// Frozen write end of some ring buffer.
///
/// Inserted items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// A free space of items removed by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
pub type FrozenProd<R> = Frozen<R, true, false>;

/// Frozen read end of some ring buffer.
///
/// A free space of removed items is not visible for an opposite write end until [`Self::commit`]/[`Self::sync`] is called or `Self` is dropped.
/// Items inserted by an opposite write end is not visible for `Self` until [`Self::sync`] is called.
pub type FrozenCons<R> = Frozen<R, false, true>;

impl<R: RbRef, const P: bool, const C: bool> Frozen<R, P, C> {
    /// Create new ring buffer cache.
    ///
    /// # Safety
    ///
    /// There must be only one instance containing the same ring buffer reference.
    pub unsafe fn new(rb: R) -> Self {
        Self {
            read: Cell::new(rb.deref().read_index()),
            write: Cell::new(rb.deref().write_index()),
            rb,
        }
    }

    pub fn observe(&self) -> Obs<R> {
        unsafe { Obs::new(self.rb.clone()) }
    }
}

impl<R: RbRef, const P: bool, const C: bool> ToRbRef for Frozen<R, P, C> {
    type RbRef = R;

    fn rb_ref(&self) -> &R {
        &self.rb
    }
    fn into_rb_ref(self) -> R {
        self.commit();
        let this = ManuallyDrop::new(self);
        unsafe { ptr::read(&this.rb) }
    }
}

impl<R: RbRef, const P: bool, const C: bool> Frozen<R, P, C> {
    /// Commit changes to the ring buffer.
    pub fn commit(&self) {
        unsafe {
            if P {
                self.rb().set_write_index(self.write.get());
            }
            if C {
                self.rb().set_read_index(self.read.get());
            }
        }
    }

    /// Fetch changes from the ring buffer.
    pub fn fetch(&self) {
        if P {
            self.read.set(self.rb().read_index());
        }
        if C {
            self.write.set(self.rb().write_index());
        }
    }

    /// Commit changes to and fetch updates from the ring buffer.
    pub fn sync(&self) {
        self.commit();
        self.fetch();
    }
}

impl<R: RbRef> FrozenProd<R> {
    /// Discard new items pushed since last sync.
    pub fn discard(&mut self) {
        let last_tail = self.rb().write_index();
        let (first, second) = unsafe { self.rb().unsafe_slices(last_tail, self.write.get()) };
        for item_mut in first.iter_mut().chain(second.iter_mut()) {
            unsafe { item_mut.assume_init_drop() };
        }
        self.write.set(last_tail);
    }
}

impl<R: RbRef, const P: bool, const C: bool> Observer for Frozen<R, P, C> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.rb().capacity()
    }

    #[inline]
    fn read_index(&self) -> usize {
        self.read.get()
    }
    #[inline]
    fn write_index(&self) -> usize {
        self.write.get()
    }

    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        self.rb().unsafe_slices(start, end)
    }
}

impl<R: RbRef> Producer for FrozenProd<R> {
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
}

impl<R: RbRef> Consumer for FrozenCons<R> {
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }
}

impl<R: RbRef, const P: bool, const C: bool> Drop for Frozen<R, P, C> {
    fn drop(&mut self) {
        self.commit();
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> io::Write for FrozenProd<R>
where
    Self: Producer<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        <Self as Producer>::write(self, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.commit();
        Ok(())
    }
}
impl<R: RbRef> fmt::Write for FrozenProd<R>
where
    Self: Producer<Item = u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        <Self as Producer>::write_str(self, s)
    }
}

#[cfg(feature = "std")]
impl<R: RbRef> io::Read for FrozenCons<R>
where
    Self: Consumer<Item = u8>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        <Self as Consumer>::read(self, buf)
    }
}
