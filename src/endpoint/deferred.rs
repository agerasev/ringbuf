//! Full endpoints with explicitly deferred publication and acquisition.
use super::Endpoint;
use crate::{
    rb::RbHandle,
    traits::utils::{add_mod, modulus, sub_mod},
    traits::*,
};
use core::{cell::Cell, mem::MaybeUninit, num::NonZeroUsize};

type Buffer<E> = <<E as Endpoint>::Handle as RbHandle>::Rb;

/// An endpoint that publishes on `commit` and acquires peer progress on `fetch`.
/// Drop commits. Forgetting may leak values, but cannot cause double destruction.
/// `E` may be an owned endpoint or an exclusive borrow of one.
pub struct Deferred<E: Endpoint, const P: bool, const C: bool> {
    inner: Option<E>,
    read: Cell<usize>,
    write: Cell<usize>,
    floor: Cell<usize>,
}
pub type DeferredProd<E> = Deferred<E, true, false>;
pub type DeferredCons<E> = Deferred<E, false, true>;

impl<E: Endpoint, const P: bool, const C: bool> Deferred<E, P, C> {
    /// Convert an immediate endpoint into a deferred endpoint.
    /// # Safety
    /// `E` must exclusively own precisely the roles indicated by `P` and `C`
    /// (exactly one must be true). It must have no unpublished changes. Any local
    /// caches must be invalidated so they recover after this endpoint is forgotten.
    pub unsafe fn from_endpoint(inner: E) -> Self {
        assert!(P != C);
        let rb = inner.rb();
        let read = if C { rb.read_claimed_index() } else { rb.read_released_index() };
        let write = rb.write_index();
        if C {
            unsafe {
                rb.set_read_released(read);
                rb.set_read_claimed(write);
            }
        }
        Self {
            inner: Some(inner),
            read: Cell::new(read),
            write: Cell::new(write),
            floor: Cell::new(if P { write } else { read }),
        }
    }

    fn buffer(&self) -> &Buffer<E> {
        self.inner.as_ref().unwrap().rb()
    }

    /// Publish changes and establish a new undo boundary, retaining this endpoint.
    pub fn commit(&mut self) {
        if P {
            self.floor.set(self.write.get());
            unsafe { self.buffer().set_write_index(self.write.get()) };
        }
        if C {
            self.floor.set(self.read.get());
            unsafe { self.buffer().set_read_released(self.read.get()) };
            self.buffer().notify_read();
        }
    }

    /// Acquire all currently available peer progress, preserving pending work.
    /// Returns the number of additional slots acquired; never waits.
    pub fn fetch(&mut self) -> usize {
        if P {
            let next = self.buffer().read_released_index();
            let count = sub_mod(next, self.read.replace(next), modulus(self));
            count
        } else {
            let next = self.buffer().write_index();
            let count = sub_mod(next, self.write.replace(next), modulus(self));
            unsafe { self.buffer().set_read_claimed(next) };
            count
        }
    }

    /// Commit, then fetch. Returns the number of newly acquired slots.
    pub fn sync(&mut self) -> usize {
        self.commit();
        self.fetch()
    }

    /// Number of operations that can still be undone.
    pub fn pending_len(&self) -> usize {
        sub_mod(if P { self.write.get() } else { self.read.get() }, self.floor.get(), modulus(self))
    }

    fn finish(&mut self) {
        if C {
            // Return the untouched suffix before releasing slots or calling user code.
            unsafe { self.buffer().set_read_claimed(self.read.get()) };
        }
        self.commit();
    }

    /// Commit and return the original endpoint or exclusive borrow.
    pub fn into_inner(mut self) -> E {
        self.finish();
        self.inner.take().unwrap()
    }
}

impl<E: Endpoint> DeferredProd<E> {
    /// Remove and return the last unpublished item.
    pub fn undo_push(&mut self) -> Option<<Self as Observer>::Item> {
        if self.write.get() == self.floor.get() {
            return None;
        }
        let index = sub_mod(self.write.get(), 1, modulus(self));
        self.write.set(index);
        Some(unsafe { self.buffer().unsafe_slices(index, index + 1).0[0].assume_init_read() })
    }

    /// Drop all unpublished items, preserving the last committed write boundary.
    /// The range is detached before any destructor runs, including during unwind.
    pub fn discard(&mut self) {
        let end = self.write.replace(self.floor.get());
        struct Cleanup<'a, B: RingBuffer + ?Sized> {
            rb: &'a B,
            next: usize,
            end: usize,
        }
        impl<B: RingBuffer + ?Sized> Cleanup<'_, B> {
            fn one(&mut self) {
                let index = self.next;
                self.next = add_mod(index, 1, modulus(self.rb));
                unsafe { self.rb.unsafe_slices_mut(index, index + 1).0[0].assume_init_drop() };
            }
        }
        impl<B: RingBuffer + ?Sized> Drop for Cleanup<'_, B> {
            fn drop(&mut self) {
                while self.next != self.end {
                    self.one();
                }
            }
        }
        let mut cleanup = Cleanup {
            rb: self.buffer(),
            next: self.floor.get(),
            end,
        };
        while cleanup.next != cleanup.end {
            cleanup.one();
        }
    }
}

impl<E: Endpoint> DeferredCons<E> {
    /// Restore a value to the last removed slot. The supplied value may differ
    /// from the original one. Returns it if there is no uncommitted removal.
    pub fn undo_pop(&mut self, value: <Self as Observer>::Item) -> Result<(), <Self as Observer>::Item> {
        if self.read.get() == self.floor.get() {
            return Err(value);
        }
        let index = sub_mod(self.read.get(), 1, modulus(self));
        unsafe { self.buffer().unsafe_slices_mut(index, index + 1).0[0].write(value) };
        self.read.set(index);
        Ok(())
    }
}

impl<E: Endpoint, const P: bool, const C: bool> Drop for Deferred<E, P, C> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            self.finish();
        }
    }
}
impl<E: Endpoint, const P: bool, const C: bool> Observer for Deferred<E, P, C> {
    type Item = <Buffer<E> as Observer>::Item;
    fn capacity(&self) -> NonZeroUsize {
        self.buffer().capacity()
    }
    fn read_index(&self) -> usize {
        self.read.get()
    }
    fn write_index(&self) -> usize {
        self.write.get()
    }
    fn retained_len(&self) -> usize {
        self.buffer().retained_len()
    }
    fn queued_len(&self) -> usize {
        self.buffer().queued_len()
    }
}
unsafe impl<E: Endpoint, const P: bool, const C: bool> RawObserver for Deferred<E, P, C> {
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.buffer().unsafe_slices(start, end) }
    }
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.buffer().unsafe_slices_mut(start, end) }
    }
}
unsafe impl<E: Endpoint> RawProducer for DeferredProd<E> {
    unsafe fn set_write_index(&self, value: usize) {
        self.write.set(value);
    }
}
unsafe impl<E: Endpoint> RawConsumer for DeferredCons<E> {
    unsafe fn set_read_index(&self, value: usize) {
        self.read.set(value);
    }
}
impl<E: Endpoint> Producer for DeferredProd<E> {}
impl<E: Endpoint> Consumer for DeferredCons<E> {}
impl<E: Endpoint, const P: bool, const C: bool> Presence for Deferred<E, P, C>
where
    Buffer<E>: Presence,
{
    fn read_is_held(&self) -> bool {
        self.buffer().read_is_held()
    }
    fn write_is_held(&self) -> bool {
        self.buffer().write_is_held()
    }
}
unsafe impl<E: Endpoint, const P: bool, const C: bool> Endpoint for Deferred<E, P, C> {
    type Handle = E::Handle;
    fn rb_handle(&self) -> &Self::Handle {
        self.inner.as_ref().unwrap().rb_handle()
    }
    fn into_rb_handle(self) -> Self::Handle {
        self.into_inner().into_rb_handle()
    }
}
crate::traits::producer::impl_producer_traits!(DeferredProd<E: Endpoint>);
crate::traits::consumer::impl_consumer_traits!(DeferredCons<E: Endpoint>);
