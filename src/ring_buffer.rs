use crate::{
    raw::{RawConsumer, RawProducer},
    Consumer, Producer,
};

/// An abstract ring buffer.
///
/// See [`RawRb`](`crate::raw::RawRb`) for details of internal implementation of the ring buffer.
///
/// This trait contains methods that takes `&mut self` allowing you to use ring buffer without splitting it into [`Producer`] and [`Consumer`].
///
/// There are `push*_overwrite` methods that cannot be used from [`Producer`].
///
/// The ring buffer can be guarded with mutex or other synchronization primitive and be used from different threads without splitting (but now only in blocking mode, obviously).
pub trait RingBuffer: Consumer + Producer
where
    Self::Raw: RawConsumer + RawProducer,
{
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
