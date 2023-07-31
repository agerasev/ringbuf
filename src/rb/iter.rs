use crate::traits::Consumer;
use core::marker::PhantomData;

/// An iterator that removes items from the ring buffer.
pub struct PopIter<R: AsMut<C> + AsRef<C>, C: Consumer> {
    target: R,
    _ghost: PhantomData<C>,
}

impl<R: AsMut<C> + AsRef<C>, C: Consumer> PopIter<R, C> {
    pub fn new(target: R) -> Self {
        Self {
            target,
            _ghost: PhantomData,
        }
    }
}

impl<R: AsMut<C> + AsRef<C>, C: Consumer> Iterator for PopIter<R, C> {
    type Item = C::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.target.as_mut().try_pop()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), None)
    }
}

impl<R: AsMut<C> + AsRef<C>, C: Consumer> ExactSizeIterator for PopIter<R, C> {
    fn len(&self) -> usize {
        self.target.as_ref().occupied_len()
    }
}
