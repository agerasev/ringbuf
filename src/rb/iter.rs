use super::traits::RbRef;
use crate::{
    halves::FrozenCons,
    traits::{Consumer, Observer},
};

/// An iterator that removes items from the ring buffer.
pub struct PopIter<R: RbRef> {
    frozen: FrozenCons<R>,
}

impl<R: RbRef> PopIter<R> {
    pub unsafe fn new(target: R) -> Self {
        Self {
            frozen: unsafe { FrozenCons::new(target) },
        }
    }
}

impl<R: RbRef> Iterator for PopIter<R> {
    type Item = <R::Target as Observer>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.frozen.try_pop()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), None)
    }
}

impl<R: RbRef> ExactSizeIterator for PopIter<R> {
    fn len(&self) -> usize {
        self.frozen.occupied_len()
    }
}
