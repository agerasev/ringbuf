use super::{utils::modulus, Observer};
#[cfg(feature = "std")]
use crate::utils::slice_assume_init_mut;
use crate::utils::write_slice;
use core::mem::MaybeUninit;
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
    unsafe fn set_write_index(&mut self, value: usize);

    /// Moves `write` pointer by `count` places forward.
    ///
    /// # Safety
    ///
    /// First `count` items in free space must be initialized.
    ///
    /// Must not be called concurrently.
    unsafe fn advance_write_index(&mut self, count: usize) {
        self.set_write_index((self.write_index() + count) % modulus(self));
    }

    /// Provides a direct access to the ring buffer vacant memory.
    ///
    /// Returns a pair of slices of uninitialized memory, the second one may be empty.
    fn vacant_slices(&self) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]) {
        let (first, second) = unsafe { self.unsafe_slices(self.write_index(), self.read_index() + self.capacity().get()) };
        (first as &_, second as &_)
    }

    /// Mutable version of [`Self::vacant_slices`].
    ///
    /// Vacant memory is uninitialized. Initialized items must be put starting from the beginning of first slice.
    /// When first slice is fully filled then items must be put to the beginning of the second slice.
    ///
    /// *This method must be followed by [`Self::advance_write`] call with the number of items being put previously as argument.*
    /// *No other mutating calls allowed before that.*
    fn vacant_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe { self.unsafe_slices(self.write_index(), self.read_index() + self.capacity().get()) }
    }

    /// Appends an item to the ring buffer.
    ///
    /// If buffer is full returns an `Err` containing the item that hasn't been appended.
    fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
        if !self.is_full() {
            unsafe {
                self.vacant_slices_mut().0.get_unchecked_mut(0).write(elem);
                self.advance_write_index(1)
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
        unsafe { self.advance_write_index(count) };
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
        unsafe { self.advance_write_index(count) };
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
        unsafe { self.advance_write_index(read_count) };
        Ok(read_count)
    }
}

#[macro_export]
macro_rules! impl_producer_traits {
    ($type:ident $(< $( $param:tt $( : $first_bound:tt $(+ $next_bound:tt )* )? ),+ >)?) => {

        #[cfg(feature = "std")]
        impl $(< $( $param $( : $first_bound $(+ $next_bound )* )? ),+ >)? std::io::Write for $type $(< $( $param ),+ >)?
        where
            Self: $crate::traits::Producer<Item = u8>,
        {
            fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
                use $crate::producer::Producer;
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

        impl $(< $( $param $( : $first_bound $(+ $next_bound )* )? ),+ >)? core::fmt::Write for $type $(< $( $param ),+ >)?
        where
            Self: $crate::traits::Producer<Item = u8>,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                use $crate::producer::Producer;
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

#[macro_export]
macro_rules! delegate_producer {
    ($ref:expr, $mut:expr) => {
        #[inline]
        unsafe fn set_write_index(&mut self, value: usize) {
            $mut(self).set_write_index(value)
        }
        #[inline]
        unsafe fn advance_write_index(&mut self, count: usize) {
            $mut(self).advance_write_index(count)
        }

        #[inline]
        fn vacant_slices(&self) -> (&[core::mem::MaybeUninit<Self::Item>], &[core::mem::MaybeUninit<Self::Item>]) {
            $ref(self).vacant_slices()
        }
        #[inline]
        fn vacant_slices_mut(&mut self) -> (&mut [core::mem::MaybeUninit<Self::Item>], &mut [core::mem::MaybeUninit<Self::Item>]) {
            $mut(self).vacant_slices_mut()
        }

        #[inline]
        fn try_push(&mut self, elem: Self::Item) -> Result<(), Self::Item> {
            $mut(self).try_push(elem)
        }

        #[inline]
        fn push_iter<I: Iterator<Item = Self::Item>>(&mut self, iter: I) -> usize {
            $mut(self).push_iter(iter)
        }

        #[inline]
        fn push_slice(&mut self, elems: &[Self::Item]) -> usize
        where
            Self::Item: Copy,
        {
            $mut(self).push_slice(elems)
        }
    };
}
