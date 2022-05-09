use core::{
    cell::UnsafeCell, convert::AsMut, marker::PhantomData, mem::MaybeUninit, num::NonZeroUsize,
};

/// Abstract container for the ring buffer.
pub trait Container<T>: AsMut<[MaybeUninit<T>]> {}
impl<T, C> Container<T> for C where C: AsMut<[MaybeUninit<T>]> {}

pub(crate) struct SharedStorage<T, C: Container<T>> {
    len: NonZeroUsize,
    container: UnsafeCell<C>,
    phantom: PhantomData<T>,
}

unsafe impl<T, C: Container<T>> Sync for SharedStorage<T, C> where T: Send {}

impl<T, C: Container<T>> SharedStorage<T, C> {
    pub fn new(mut container: C) -> Self {
        Self {
            len: NonZeroUsize::new(container.as_mut().len()).unwrap(),
            container: UnsafeCell::new(container),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        self.len
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_slice(&self) -> &mut [MaybeUninit<T>] {
        (&mut *self.container.get()).as_mut()
    }
}
