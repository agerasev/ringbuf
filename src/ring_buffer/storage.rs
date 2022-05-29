use core::{
    cell::UnsafeCell, convert::AsMut, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Abstract container for the ring buffer.
///
/// # Safety
///
/// *Container must not cause data race on concurrent `.as_mut()` calls.*
///
/// For now there are safe ring buffer constructors only for `Vec` and `[T; N]`.
/// Using other conainers is unsafe.
pub trait Container<T> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>];
}

impl<'a, T> Container<T> for &'a mut [MaybeUninit<T>] {
    fn len(&self) -> usize {
        <[_]>::len(self)
    }
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>] {
        self
    }
}
impl<T, const N: usize> Container<T> for [MaybeUninit<T>; N] {
    fn len(&self) -> usize {
        N
    }
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_mut()
    }
}
#[cfg(feature = "alloc")]
impl<T> Container<T> for Vec<MaybeUninit<T>> {
    fn len(&self) -> usize {
        self.len()
    }
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_mut()
    }
}

pub(crate) struct SharedStorage<T, C: Container<T>> {
    len: NonZeroUsize,
    container: UnsafeCell<C>,
    phantom: PhantomData<T>,
}

unsafe impl<T, C: Container<T>> Sync for SharedStorage<T, C> where T: Send {}

impl<T, C: Container<T>> SharedStorage<T, C> {
    pub fn new(container: C) -> Self {
        Self {
            len: NonZeroUsize::new(container.len()).unwrap(),
            container: UnsafeCell::new(container),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        self.len
    }

    /// Returns underlying raw ring buffer memory as slice.
    ///
    /// # Safety
    ///
    /// All operations on this data must cohere with the counter.
    ///
    /// **Accessing raw data is extremely unsafe.**
    /// It is recommended to use [`Consumer::as_slices`](`crate::LocalConsumer::as_slices`) and
    /// [`Producer::free_space_as_slices`](`crate::LocalProducer::free_space_as_slices`) instead.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_slice(&self) -> &mut [MaybeUninit<T>] {
        (&mut *self.container.get()).as_mut_slice()
    }

    pub fn into_inner(self) -> C {
        self.container.into_inner()
    }
}
