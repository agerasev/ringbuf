#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    consumer::Consumer,
    index::Index,
    producer::Producer,
    ref_::{ArcFamily, Family, RcFamily, RefFamily},
    storage::{Shared, Static, Storage},
    traits::{Observer, RingBuffer, Split},
};
use alloc::rc::Rc;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::Range,
    ptr,
};

/// Modulus for pointers to item in ring buffer storage.
///
/// Equals to `2 * capacity`.
#[inline]
fn modulus(this: &impl Observer) -> NonZeroUsize {
    unsafe { NonZeroUsize::new_unchecked(2 * this.capacity().get()) }
}

/// Returns a pair of ranges between `start` and `end` indices in a ring buffer with specific `capacity`.
///
/// `start` and `end` may be arbitrary large, but must satisfy the following condition: `0 <= (start - end) % (2 * capacity) <= capacity`.
/// Actual indices are taken modulo `capacity`.
///
/// The first range starts from `start`. If the first slice is empty then second slice is empty too.
fn ranges(capacity: NonZeroUsize, start: usize, end: usize) -> (Range<usize>, Range<usize>) {
    let (head_quo, head_rem) = (start / capacity, start % capacity);
    let (tail_quo, tail_rem) = (end / capacity, end % capacity);

    if (head_quo + tail_quo) % 2 == 0 {
        (head_rem..tail_rem, 0..0)
    } else {
        (head_rem..capacity.get(), 0..tail_rem)
    }
}

/// Ring buffer that could be shared between threads.
///
/// Note that there is no explicit requirement of `T: Send`. Instead [`Rb`] will work just fine even with `T: !Send`
/// until you try to send its [`Producer`] or [`Consumer`] to another thread.
#[cfg_attr(
    feature = "std",
    doc = r##"
```
use std::thread;
use ringbuf::{Rb, storage::Heap, traits::*};

let rb = Rb::<Heap<i32>>::new(256);
let (mut prod, mut cons) = rb.split();
thread::spawn(move || {
    prod.try_push(123).unwrap();
})
.join();
thread::spawn(move || {
    assert_eq!(cons.try_pop().unwrap(), 123);
})
.join();
```
"##
)]
pub struct Rb<S: Storage, R: Index, W: Index> {
    storage: Shared<S>,
    read: R,
    write: W,
}

impl<S: Storage, R: Index, W: Index> Rb<S, R, W> {
    /// Constructs ring buffer from storage and indices.
    ///
    /// # Safety
    ///
    /// The items in storage inside `read..write` range must be initialized, items outside this range must be uninitialized.
    /// `read` and `write` positions must be valid (see [`RbBase`](`crate::ring_buffer::RbBase`)).
    pub unsafe fn from_raw_parts(storage: S, read: R, write: W) -> Self {
        Self {
            storage: Shared::new(storage),
            read,
            write,
        }
    }
    /// Destructures ring buffer into underlying storage and `read` and `write` indices.
    ///
    /// # Safety
    ///
    /// Initialized contents of the storage must be properly dropped.
    pub unsafe fn into_raw_parts(self) -> (S, R, W) {
        let this = ManuallyDrop::new(self);
        (
            ptr::read(&this.storage).into_inner(),
            ptr::read(&this.read),
            ptr::read(&this.write),
        )
    }

    pub unsafe fn slices(
        &self,
        start: usize,
        end: usize,
    ) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        let (first, second) = ranges(self.capacity(), start, end);
        (self.storage.slice(first), self.storage.slice(second))
    }
    pub unsafe fn read_index(&self) -> &R {
        &self.read
    }
    pub unsafe fn write_index(&self) -> &W {
        &self.write
    }
}

impl<S: Storage, R: Index, W: Index> Observer for Rb<S, R, W> {
    type Item = S::Item;

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.storage.len()
    }

    fn occupied_len(&self) -> usize {
        let modulus = modulus(self);
        (modulus.get() + self.write.get() - self.read.get()) % modulus
    }
    fn vacant_len(&self) -> usize {
        let modulus = modulus(self);
        (self.capacity().get() + self.read.get() - self.write.get()) % modulus
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.read.get() == self.write.get()
    }
}

impl<S: Storage, R: Index, W: Index> Producer for Rb<S, R, W> {
    #[inline]
    unsafe fn advance_write(&self, count: usize) {
        self.write.set((self.write.get() + count) % modulus(self));
    }

    #[inline]
    unsafe fn unsafe_vacant_slices(
        &self,
    ) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        unsafe { self.slices(self.write.get(), self.read.get() + self.capacity().get()) }
    }
}

impl<S: Storage, R: Index, W: Index> Consumer for Rb<S, R, W> {
    #[inline]
    unsafe fn advance_read(&self, count: usize) {
        self.read.set((self.read.get() + count) % modulus(self));
    }

    #[inline]
    unsafe fn unsafe_occupied_slices(
        &self,
    ) -> (&mut [MaybeUninit<S::Item>], &mut [MaybeUninit<S::Item>]) {
        self.slices(self.read.get(), self.write.get())
    }
}

impl<S: Storage, R: Index, W: Index> RingBuffer for Rb<S, R, W> {}

impl<S: Storage, R: Index, W: Index> Drop for Rb<S, R, W> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Producer wrapper of ring buffer.
pub struct Prod<A: Family, S: Storage, R: Index, W: Index> {
    base: A::Ref<Rb<S, R, W>>,
}
/*
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

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    #[inline]
    fn occupied_len(&self) -> usize {
        self.base.occupied_len()
    }
    #[inline]
    fn vacant_len(&self) -> usize {
        self.base.vacant_len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.base.is_empty()
    }
    #[inline]
    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
impl<R: Deref> Producer for Prod<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn advance_write(&self, count: usize) {
        self.base.advance_write(count);
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
impl_prod_traits!(Prod);
*/

/// Producer wrapper of ring buffer.
pub struct Cons<A: Family, S: Storage, R: Index, W: Index> {
    base: A::Ref<Rb<S, R, W>>,
}

/*
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

    #[inline]
    fn capacity(&self) -> NonZeroUsize {
        self.base.capacity()
    }

    #[inline]
    fn occupied_len(&self) -> usize {
        self.base.occupied_len()
    }
    #[inline]
    fn vacant_len(&self) -> usize {
        self.base.vacant_len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.base.is_empty()
    }
    #[inline]
    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
impl<R: Deref> Consumer for Cons<R>
where
    R::Target: RingBuffer,
{
    #[inline]
    unsafe fn advance_read(&self, count: usize) {
        self.base.advance_read(count)
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
impl_cons_traits!(Cons);
*/
/*
impl<'a, S: Storage, R: Index, W: Index>
    Split<S::Item, Prod<RefFamily<'a>, S, R, W>, Cons<RefFamily<'a>, S, R, W>>
    for &'a mut Rb<S, R, W>
{
    fn split(self) -> (Prod<RefFamily<'a>, S, R, W>, Cons<RefFamily<'a>, S, R, W>) {
        unsafe { (Prod::new(self), Cons::new(self)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage, R: Index, W: Index>
    Split<S::Item, Prod<RcFamily, S, R, W>, Cons<RcFamily, S, R, W>> for Rb<S, R, W>
{
    fn split(self) -> (Prod<RcFamily, S, R, W>, Cons<RcFamily, S, R, W>) {
        let rc = Rc::new(self);
        unsafe { (Prod::new(rc.clone()), Cons::new(rc)) }
    }
}
#[cfg(feature = "alloc")]
impl<S: Storage, R: Index, W: Index>
    Split<S::Item, Prod<ArcFamily, S, R, W>, Cons<ArcFamily, S, R, W>> for Rb<S, R, W>
{
    fn split(self) -> (Prod<ArcFamily, S, R, W>, Cons<ArcFamily, S, R, W>) {
        let arc = Arc::new(self);
        unsafe { (Prod::new(arc.clone()), Cons::new(arc)) }
    }
}
*/
impl<T, R: Index + Default, W: Index + Default, const N: usize> Default for Rb<Static<T, N>, R, W> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(crate::utils::uninit_array(), R::default(), W::default()) }
    }
}

#[cfg(feature = "alloc")]
impl<T, R: Index + Default, W: Index + Default> Rb<Heap<T>, R, W> {
    /// Creates a new instance of a ring buffer.
    ///
    /// *Panics if allocation failed or `capacity` is zero.*
    pub fn new(capacity: usize) -> Self {
        Self::try_new(capacity).unwrap()
    }
    /// Creates a new instance of a ring buffer returning an error if allocation failed.
    ///
    /// *Panics if `capacity` is zero.*
    pub fn try_new(capacity: usize) -> Result<Self, alloc::collections::TryReserveError> {
        let mut data = alloc::vec::Vec::new();
        data.try_reserve_exact(capacity)?;
        data.resize_with(capacity, core::mem::MaybeUninit::uninit);
        Ok(unsafe { Self::from_raw_parts(data, R::default(), W::default()) })
    }
}
