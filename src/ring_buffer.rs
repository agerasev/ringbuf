use crate::{
    consumer::{self, Consumer},
    producer::{self, Producer},
    raw::RawRb,
};
#[cfg(feature = "alloc")]
use alloc::{rc::Rc, sync::Arc};

/// An abstract ring buffer.
///
/// See [`RawRb`](`crate::raw::RawRb`) for details of internal implementation of the ring buffer.
///
/// This trait contains methods that takes `&mut self` allowing you to use ring buffer without splitting it into [`Producer`] and [`Consumer`].
///
/// There are `push*_overwrite` methods that cannot be used from [`Producer`].
///
/// The ring buffer can be guarded with mutex or other synchronization primitive and be used from different threads without splitting
/// (but only in blocking mode, obviously).
pub trait RingBuffer: Consumer + Producer {
    /// Pushes an item to the ring buffer overwriting the latest item if the buffer is full.
    ///
    /// Returns overwritten item if overwriting took place.
    fn push_overwrite(&mut self, elem: Self::Item) -> Option<Self::Item> {
        let ret = if self.is_full() { self.try_pop() } else { None };
        let _ = self.try_push(elem);
        ret
    }

    /// Appends items from an iterator to the ring buffer.
    ///
    /// *This method consumes iterator until its end.*
    /// Exactly last `min(iter.len(), capacity)` items from the iterator will be stored in the ring buffer.
    fn push_iter_overwrite<I: Iterator<Item = Self::Item>>(&mut self, iter: I) {
        for elem in iter {
            self.push_overwrite(elem);
        }
    }

    /// Appends items from slice to the ring buffer overwriting existing items in the ring buffer.
    ///
    /// If the slice length is greater than ring buffer capacity then only last `capacity` items from slice will be stored in the buffer.
    fn push_slice_overwrite(&mut self, elems: &[Self::Item])
    where
        Self::Item: Copy,
    {
        if elems.len() > self.vacant_len() {
            self.skip(usize::min(
                elems.len() - self.vacant_len(),
                self.occupied_len(),
            ));
        }
        self.push_slice(if elems.len() > self.vacant_len() {
            &elems[(elems.len() - self.vacant_len())..]
        } else {
            elems
        });
    }
}

pub trait Split<R> {
    fn split(self) -> (producer::Wrap<R>, consumer::Wrap<R>);
}

impl<'a, R: RingBuffer + RawRb> Split<&'a R> for &'a mut R {
    fn split(self) -> (producer::Wrap<&'a R>, consumer::Wrap<&'a R>) {
        unsafe { (producer::Wrap::new(self), consumer::Wrap::new(self)) }
    }
}

#[cfg(feature = "alloc")]
impl<R: RingBuffer + RawRb> Split<Arc<R>> for R {
    fn split(self) -> (producer::Wrap<Arc<R>>, consumer::Wrap<Arc<R>>) {
        let arc = Arc::new(self);
        unsafe { (producer::Wrap::new(arc.clone()), consumer::Wrap::new(arc)) }
    }
}

#[cfg(feature = "alloc")]
impl<R: RingBuffer + RawRb> Split<Rc<R>> for R {
    fn split(self) -> (producer::Wrap<Rc<R>>, consumer::Wrap<Rc<R>>) {
        let arc = Rc::new(self);
        unsafe { (producer::Wrap::new(arc.clone()), consumer::Wrap::new(arc)) }
    }
}
