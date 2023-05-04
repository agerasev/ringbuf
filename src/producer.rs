#[cfg(feature = "std")]
use crate::utils::slice_assume_init_mut;
use crate::{
    //cached::CachedProd,
    observer::Observer,
    traits::RingBuffer,
    utils::write_slice,
};
use core::{mem::MaybeUninit, num::NonZeroUsize, ops::Deref};
#[cfg(feature = "std")]
use std::{
    cmp,
    io::{self, Read},
};

/// Producer part of ring buffer.
///
/// # Mode
///
/// It can operate in immediate (by default) or postponed mode.
/// Mode could be switched using [`Self::postponed`]/[`Self::into_postponed`] and [`Self::into_immediate`] methods.
///
/// + In immediate mode removed and inserted items are automatically synchronized with the other end.
/// + In postponed mode synchronization occurs only when [`Self::sync`] or [`Self::into_immediate`] is called or when `Self` is dropped.
///   The reason to use postponed mode is that multiple subsequent operations are performed faster due to less frequent cache synchronization.
pub trait Producer: Observer {
    /// Moves `write` pointer by `count` places forward.
    ///
    /// # Safety
    ///
    /// First `count` items in free space must be initialized.
    ///
    /// Must not be called concurrently.
    unsafe fn advance_write(&self, count: usize);

    unsafe fn unsafe_vacant_slices(
        &self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    );

    /// Provides a direct access to the ring buffer vacant memory.
    ///
    /// Returns a pair of slices of uninitialized memory, the second one may be empty.
    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let (first, second) = unsafe { self.unsafe_vacant_slices() };
        (first as &_, second as &_)
    }

    /// Mutable version of [`Self::vacant_slices`].
    ///
    /// Vacant memory is uninitialized. Initialized items must be put starting from the beginning of first slice.
    /// When first slice is fully filled then items must be put to the beginning of the second slice.
    ///
    /// *This method must be followed by [`Self::advance_write`] call with the number of items being put previously as argument.*
    /// *No other mutating calls allowed before that.*
    fn vacant_slices_mut(
        &mut self,
    ) -> (
        &mut [MaybeUninit<Self::Item>],
        &mut [MaybeUninit<Self::Item>],
    ) {
        unsafe { self.unsafe_vacant_slices() }
    }

    /// Appends an item to the ring buffer.
    ///
    /// If buffer is full returns an `Err` containing the item that hasn't been appended.
    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        if !self.is_full() {
            unsafe {
                self.vacant_slices_mut().0.get_unchecked_mut(0).write(elem);
                self.advance_write(1)
            };
            Ok(())
        } else {
            Err(elem)
        }
    }

    /// Appends items from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    ///
    /// Returns count of items been appended to the ring buffer.
    ///
    /// *Inserted items are committed to the ring buffer all at once in the end,*
    /// *e.g. when buffer is full or iterator has ended.*
    fn push_iter<I: Iterator<Item = Self::Item>>(&mut self, mut iter: I) -> usize {
        let (left, right) = self.vacant_slices_mut();
        let mut count = 0;
        for place in left.iter_mut().chain(right.iter_mut()) {
            match iter.next() {
                Some(elem) => unsafe { place.as_mut_ptr().write(elem) },
                None => break,
            }
            count += 1;
        }
        unsafe { self.advance_write(count) };
        count
    }

    /// Appends items from slice to the ring buffer.
    ///
    /// Returns count of items been appended to the ring buffer.
    fn push_slice(&mut self, elems: &[Self::Item]) -> usize
    where
        Self::Item: Copy,
    {
        let (left, right) = self.vacant_slices_mut();
        let count = if elems.len() < left.len() {
            write_slice(&mut left[..elems.len()], elems);
            elems.len()
        } else {
            let (left_elems, elems) = elems.split_at(left.len());
            write_slice(left, left_elems);
            left.len()
                + if elems.len() < right.len() {
                    write_slice(&mut right[..elems.len()], elems);
                    elems.len()
                } else {
                    write_slice(right, &elems[..right.len()]);
                    right.len()
                }
        };
        unsafe { self.advance_write(count) };
        count
    }

    #[cfg(feature = "std")]
    /// Reads at most `count` bytes from `Read` instance and appends them to the ring buffer.
    /// If `count` is `None` then as much as possible bytes will be read.
    ///
    /// Returns `Ok(n)` if `read` succeeded. `n` is number of bytes been read.
    /// `n == 0` means that either `read` returned zero or ring buffer is full.
    ///
    /// If `read` is failed then original error is returned. In this case it is guaranteed that no items was read from the reader.
    /// To achieve this we read only one contiguous slice at once. So this call may read less than `remaining` items in the buffer even if the reader is ready to provide more.
    fn read_from<S: Read>(&mut self, reader: &mut S, count: Option<usize>) -> io::Result<usize>
    where
        Self: Producer<Item = u8>,
    {
        let (left, _) = self.vacant_slices_mut();
        let count = cmp::min(count.unwrap_or(left.len()), left.len());
        let left_init = unsafe { slice_assume_init_mut(&mut left[..count]) };

        let read_count = reader.read(left_init)?;
        assert!(read_count <= count);
        unsafe { self.advance_write(read_count) };
        Ok(read_count)
    }
}

macro_rules! impl_prod_traits {
    ($Prod:ident) => {
        #[cfg(feature = "std")]
        impl<R: core::ops::Deref> std::io::Write for $Prod<R>
        where
            R::Target: crate::traits::RingBuffer<Item = u8>,
        {
            fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
                use crate::producer::Producer;
                let n = self.push_slice(buffer);
                if n == 0 && !buffer.is_empty() {
                    Err(std::io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(n)
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<R: core::ops::Deref> core::fmt::Write for $Prod<R>
        where
            R::Target: crate::traits::RingBuffer<Item = u8>,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                use crate::producer::Producer;
                let n = self.push_slice(s.as_bytes());
                if n != s.len() {
                    Err(core::fmt::Error::default())
                } else {
                    Ok(())
                }
            }
        }
    };
}
pub(crate) use impl_prod_traits;

/*
impl<R: Deref> Prod<R>
where
    R::Target: RingBuffer,
{
    pub fn cached(&mut self) -> CachedProd<&R::Target> {
        unsafe { CachedProd::new(&self.base) }
    }
    pub fn into_cached(self) -> CachedProd<R> {
        unsafe { CachedProd::new(self.base) }
    }
}
*/
