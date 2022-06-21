use core::{
    cell::UnsafeCell, convert::AsMut, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Abstract container for the ring buffer.
///
/// Container items must be stored as a contiguous array.
///
/// # Safety
///
/// *[`Self::len`]/[`Self::is_empty`] must always return the same value.*
///
/// *Container must not cause data race on concurrent [`Self::as_mut_slice`]/[`Self::as_mut_ptr`] calls.*
pub unsafe trait Container<T> {
    /// Length of the container.
    fn len(&self) -> usize;

    /// Checks if container is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return slice of all container items.
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>];

    /// Return pointer to the beginning of the container items.
    #[inline]
    fn as_mut_ptr(&mut self) -> *mut MaybeUninit<T> {
        self.as_mut_slice().as_mut_ptr()
    }
}

unsafe impl<'a, T> Container<T> for &'a mut [MaybeUninit<T>] {
    #[inline]
    fn len(&self) -> usize {
        <[_]>::len(self)
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>] {
        self
    }
}

unsafe impl<T, const N: usize> Container<T> for [MaybeUninit<T>; N] {
    #[inline]
    fn len(&self) -> usize {
        N
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_mut()
    }
}
#[cfg(feature = "alloc")]
unsafe impl<T> Container<T> for Vec<MaybeUninit<T>> {
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_mut()
    }
}

pub(crate) struct SharedStorage<T, C: Container<T>> {
    container: UnsafeCell<C>,
    _phantom: PhantomData<T>,
}

unsafe impl<T, C: Container<T>> Sync for SharedStorage<T, C> where T: Send {}

impl<T, C: Container<T>> SharedStorage<T, C> {
    /// Create new storage.
    ///
    /// *Panics if container is empty.*
    pub fn new(container: C) -> Self {
        assert!(!container.is_empty());
        Self {
            container: UnsafeCell::new(container),
            _phantom: PhantomData,
        }
    }

    /// Get the length of the container.
    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked((&*self.container.get()).len()) }
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
    #[inline]
    pub unsafe fn as_slice(&self) -> &mut [MaybeUninit<T>] {
        (&mut *self.container.get()).as_mut_slice()
    }

    /// Returns underlying container.
    pub fn into_inner(self) -> C {
        self.container.into_inner()
    }
}
