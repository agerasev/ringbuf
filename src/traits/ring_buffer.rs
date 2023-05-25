use super::{Consumer, Observer, Producer};
use core::mem::MaybeUninit;

/// An abstract ring buffer.
///
/// # Details
///
/// The ring buffer consists of an array (of `capacity` size) and two indices: `read` and `write`.
/// When an item is extracted from the ring buffer it is taken from the `read` index and after that `read` is incremented.
/// New item is appended to the `write` index and `write` is incremented after that.
///
/// The `read` and `write` indices are modulo `2 * capacity` (not just `capacity`).
/// It allows us to distinguish situations when the buffer is empty (`read == write`) and when the buffer is full (`write - read` modulo `2 * capacity` equals to `capacity`)
/// without using the space for an extra element in container.
/// And obviously we cannot store more than `capacity` items in the buffer, so `write - read` modulo `2 * capacity` is not allowed to be greater than `capacity`.
pub trait RingBuffer: Observer + Consumer + Producer {
    fn read_index(&self) -> usize;
    fn write_index(&self) -> usize;

    unsafe fn set_read_index(&self, value: usize);
    unsafe fn set_write_index(&self, value: usize);

    unsafe fn unsafe_slices(
        &self,
        start: usize,
        end: usize,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    );

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
