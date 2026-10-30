//! Low-level implementation contract used by safe ring buffer operations.
use super::{
    Based, Observer,
    utils::{add_mod, modulus},
};
use core::mem::MaybeUninit;

/// Raw storage, cursor, and endpoint-ownership operations.
///
/// Implementing the safe [`super::Producer`] and [`super::Consumer`] interfaces
/// requires an explicit unsafe implementation of this contract.
///
/// # Safety
///
/// Capacity, indices, slices, and their overrides must describe the same live
/// storage. Capacity is constant and in `1..=usize::MAX / 2`; indices are below
/// twice the capacity. Raw slices must cover exactly the requested modular range;
/// a nonempty range must have a nonempty first slice.
/// Endpoint caches may conservatively underestimate available space or items,
/// but must never expose slots owned by the opposite endpoint.
///
/// All overrides of safe producer, consumer, and ring-buffer methods must preserve
/// their documented bounds, initialization, and transfer guarantees. In particular,
/// predicates, lengths, and slices used together by a default method must be consistent.
/// Valid raw slice access and capacity/index accessors must not unwind. Cursor
/// setters must publish the change before returning or unwinding, including when
/// a subsequent notification panics.
///
/// A type implementing `Consumer` must retain consumer rights for every data
/// view borrowed from it; a type implementing `Producer` must similarly retain
/// producer rights. A type implementing `RingBuffer` must grant exclusive access
/// to all storage through `&mut self`, even if earlier borrowed endpoints were
/// forgotten. Safe interfaces must not allow a competing role to be acquired
/// while a view is live. Sharing an endpoint that exposes `&Item` also requires
/// `Item: Sync`.
///
/// Ownership flags must atomically check and replace their previous state when
/// shared across threads. Raw access does not itself acquire endpoint rights:
/// callers must establish those rights and keep them for the duration of access.
///
/// A safe observer alone cannot implement a consumer:
///
/// ```compile_fail,E0277
/// use core::num::NonZeroUsize;
/// use ringbuf::traits::{Consumer, Observer};
/// struct Lying;
/// impl Observer for Lying {
///     type Item = String;
///     fn capacity(&self) -> NonZeroUsize { NonZeroUsize::new(1).unwrap() }
///     fn read_index(&self) -> usize { 0 }
///     fn write_index(&self) -> usize { 1 }
///     fn read_is_held(&self) -> bool { false }
///     fn write_is_held(&self) -> bool { false }
/// }
/// impl Consumer for Lying {}
/// Lying.try_pop();
/// ```
///
/// Nor does a safe observer satisfy the producer implementation contract:
///
/// ```compile_fail,E0277
/// use core::num::NonZeroUsize;
/// use ringbuf::traits::{Observer, Producer};
/// struct Lying;
/// impl Observer for Lying {
///     type Item = u8;
///     fn capacity(&self) -> NonZeroUsize { NonZeroUsize::new(1).unwrap() }
///     fn read_index(&self) -> usize { 0 }
///     fn write_index(&self) -> usize { 0 }
///     fn read_is_held(&self) -> bool { false }
///     fn write_is_held(&self) -> bool { false }
/// }
/// impl Producer for Lying {}
/// Lying.try_push(1).unwrap();
/// ```
pub unsafe trait Raw: Observer {
    /// Borrow the storage between two modular indices.
    ///
    /// # Safety
    /// The forward distance modulo twice the capacity must be at most capacity.
    /// Indices are interpreted modulo twice the capacity. The caller must own the corresponding endpoint
    /// rights (or the buffer exclusively) for the entire borrow. No writer may
    /// access these slots while borrowed; non-`Sync` items must not be shared
    /// between threads. Reading an item also requires that it is initialized.
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]);

    /// Mutably borrow the storage between two modular indices.
    ///
    /// # Safety
    /// The same bounds and ownership requirements as [`Self::unsafe_slices`]
    /// apply. No overlapping reference may be used during the borrow.
    #[allow(clippy::mut_from_ref)]
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]);

    /// Publish the consumer's new read index.
    ///
    /// # Safety
    /// `value` must be below twice the capacity. The caller must own consumer
    /// rights. Advance only over items already moved out or destroyed, with no
    /// outstanding access to the released slots.
    unsafe fn set_read_index(&self, value: usize);

    /// Publish the producer's new write index.
    ///
    /// # Safety
    /// `value` must be below twice the capacity. The caller must own producer
    /// rights. Advance only over initialized vacant slots, with no outstanding
    /// mutable access to the published items.
    unsafe fn set_write_index(&self, value: usize);

    /// Advance the read index by `count` slots.
    /// # Safety
    /// `count` must not exceed the occupied length, and the requirements of
    /// [`Self::set_read_index`] must hold for all released slots.
    unsafe fn advance_read_index(&self, count: usize) {
        unsafe { self.set_read_index(add_mod(self.read_index(), count, modulus(self))) };
    }

    /// Advance the write index by `count` slots.
    /// # Safety
    /// `count` must not exceed the vacant length, and the requirements of
    /// [`Self::set_write_index`] must hold for all published slots.
    unsafe fn advance_write_index(&self, count: usize) {
        unsafe { self.set_write_index(add_mod(self.write_index(), count, modulus(self))) };
    }

    /// Borrow the occupied slots mutably without assuming initialization.
    /// # Safety
    /// The caller must own consumer rights. Removed or destroyed items must be
    /// released before any further safe operation; remaining items must stay
    /// initialized. No other access to these slots may overlap this borrow.
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.unsafe_slices_mut(self.read_index(), self.write_index()) }
    }

    /// Replace the consumer-held flag, returning its previous value.
    /// # Safety
    /// Clearing the flag requires ownership of that role and the end of all its
    /// data accesses. Setting an already-set flag does not acquire another role.
    unsafe fn hold_read(&self, flag: bool) -> bool;

    /// Replace the producer-held flag, returning its previous value.
    /// # Safety
    /// Clearing the flag requires ownership of that role and the end of all its
    /// data accesses. Setting an already-set flag does not acquire another role.
    unsafe fn hold_write(&self, flag: bool) -> bool;
}

/// Opt in to forwarding the raw implementation contract to a base object.
///
/// # Safety
/// Both [`Based`] accessors must always refer to the same live base. Observer and
/// producer/consumer overrides must preserve its bounds and ownership guarantees.
/// The wrapper must satisfy the entire [`Raw`] contract for every safe interface
/// it implements. Forwarding raw operations grants no additional endpoint rights
/// and does not turn an endpoint into an exclusively owned `RingBuffer`.
/// Delegation must not duplicate endpoint rights or introduce additional storage
/// aliases beyond those available through the base interface.
///
/// Safe delegation cannot bypass the raw implementation obligation:
///
/// ```compile_fail,E0277
/// use ringbuf::traits::{Based, Consumer,
///     observer::DelegateObserver, consumer::DelegateConsumer};
/// struct Wrapper<B>(B);
/// impl<B> Based for Wrapper<B> {
///     type Base = B;
///     fn base(&self) -> &Self::Base { &self.0 }
///     fn base_mut(&mut self) -> &mut Self::Base { &mut self.0 }
/// }
/// impl<B: Consumer> DelegateObserver for Wrapper<B> {}
/// impl<B: Consumer> DelegateConsumer for Wrapper<B> {}
/// ```
pub unsafe trait DelegateRaw: Based
where
    Self::Base: Raw,
{
}

// SAFETY: DelegateRaw requires faithful delegation and consistent overrides.
unsafe impl<D: DelegateRaw + Observer<Item = <D::Base as Observer>::Item>> Raw for D
where
    D::Base: Raw,
{
    #[inline]
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        unsafe { self.base().unsafe_slices(start, end) }
    }
    #[inline]
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.base().unsafe_slices_mut(start, end) }
    }
    #[inline]
    unsafe fn set_read_index(&self, value: usize) {
        unsafe { self.base().set_read_index(value) };
    }
    #[inline]
    unsafe fn set_write_index(&self, value: usize) {
        unsafe { self.base().set_write_index(value) };
    }
    #[inline]
    unsafe fn advance_read_index(&self, count: usize) {
        unsafe { self.base().advance_read_index(count) };
    }
    #[inline]
    unsafe fn advance_write_index(&self, count: usize) {
        unsafe { self.base().advance_write_index(count) };
    }
    #[inline]
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.base_mut().occupied_slices_mut() }
    }
    #[inline]
    unsafe fn hold_read(&self, flag: bool) -> bool {
        unsafe { self.base().hold_read(flag) }
    }
    #[inline]
    unsafe fn hold_write(&self, flag: bool) -> bool {
        unsafe { self.base().hold_write(flag) }
    }
}
