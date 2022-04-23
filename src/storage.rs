use core::{
    cell::UnsafeCell,
    convert::{AsMut, AsRef},
    marker::PhantomData,
};

pub trait Container<U>: AsRef<[U]> + AsMut<[U]> {}
impl<U, C> Container<U> for C where C: AsRef<[U]> + AsMut<[U]> {}

pub struct Storage<U, C: Container<U>> {
    len: usize,
    container: UnsafeCell<C>,
    phantom: PhantomData<U>,
}

unsafe impl<U, C: Container<U>> Sync for Storage<U, C> {}

impl<U, C> Storage<U, C>
where
    C: AsRef<[U]> + AsMut<[U]>,
{
    pub fn new(mut container: C) -> Self {
        Self {
            len: container.as_mut().len(),
            container: UnsafeCell::new(container),
            phantom: PhantomData,
        }
    }

    pub fn into_inner(self) -> C {
        self.container.into_inner()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub unsafe fn as_slice(&self) -> &[U] {
        (&*self.container.get()).as_ref()
    }

    pub unsafe fn as_mut_slice(&self) -> &mut [U] {
        (&mut *self.container.get()).as_mut()
    }
}
