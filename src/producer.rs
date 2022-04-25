use alloc::sync::Arc;
use core::{
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::Deref,
    ptr::copy_nonoverlapping,
    slice,
    sync::atomic::Ordering,
};
#[cfg(feature = "std")]
use std::io::{self, Read, Write};

use crate::ring_buffer::{Container, RingBuffer, RingBufferRef};

/// Producer part of ring buffer.
pub struct Producer<T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    rb: R,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C, R> Producer<T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    pub(crate) fn new(rb: R) -> Self {
        Self {
            rb,
            _phantom: PhantomData,
        }
    }
}

impl<T, C, R> Producer<T, C, R>
where
    C: Container<MaybeUninit<T>>,
    R: RingBufferRef<T, C>,
{
    /// Returns capacity of the ring buffer.
    ///
    /// The capacity of the buffer is constant.
    pub fn capacity(&self) -> usize {
        self.rb.capacity()
    }

    /// Checks if the ring buffer is empty.
    ///
    /// The result is relevant until you push items to the producer.
    pub fn is_empty(&self) -> bool {
        self.rb.is_empty()
    }

    /// Checks if the ring buffer is full.
    ///
    /// *The result may become irrelevant at any time because of concurring activity of the consumer.*
    pub fn is_full(&self) -> bool {
        self.rb.is_full()
    }

    /// The length of the data stored in the buffer.
    ///
    /// Actual length may be equal to or less than the returned value.
    pub fn len(&self) -> usize {
        self.rb.occupied_len()
    }

    /// The remaining space in the buffer.
    ///
    /// Actual remaining space may be equal to or greater than the returning value.
    pub fn remaining(&self) -> usize {
        self.rb.vacant_len()
    }

    /// Appends an element to the ring buffer.
    ///
    /// On failure returns an `Err` containing the element that hasn't been appended.
    pub fn push(&mut self, elem: T) -> Result<(), T> {
        let (left, _) = unsafe { self.rb.vacant_slices() };
        match left.iter_mut().next() {
            Some(place) => {
                unsafe { place.as_mut_ptr().write(elem) };
                unsafe { self.rb.move_tail(1) };
                Ok(())
            }
            None => Err(elem),
        }
    }

    /// Appends elements from an iterator to the ring buffer.
    /// Elements that haven't been added to the ring buffer remain in the iterator.
    ///
    /// Returns count of elements been appended to the ring buffer.
    ///
    /// *Inserted elements are commited to the ring buffer all at once in the end,*
    /// *e.g. when buffer is full or iterator has ended.*
    pub fn push_iter<I: Iterator<Item = T>>(&mut self, iter: &mut I) -> usize {
        let (left, right) = unsafe { self.rb.vacant_slices() };
        let mut count = 0;
        for place in left.iter_mut().chain(right.iter_mut()) {
            match iter.next() {
                Some(elem) => unsafe { place.as_mut_ptr().write(elem) },
                None => break,
            }
            count += 1;
        }
        unsafe { self.rb.move_tail(count) };
        count
    }
    /*
    /// Removes at most `count` elements from the consumer and appends them to the producer.
    /// If `count` is `None` then as much as possible elements will be moved.
    /// The producer and consumer parts may be of different buffers as well as of the same one.
    ///
    /// On success returns number of elements been moved.
    pub fn move_from(&mut self, other: &mut Consumer<T>, count: Option<usize>) -> usize {
        move_items(other, self, count)
    }
    */
}
/*
impl<T: Sized + Copy> Producer<T> {
    /// Appends elements from slice to the ring buffer.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// Returns count of elements been appended to the ring buffer.
    pub fn push_slice(&mut self, elems: &[T]) -> usize {
        unsafe { self.push_copy(&*(elems as *const [T] as *const [MaybeUninit<T>])) }
    }
}

#[cfg(feature = "std")]
impl Producer<u8> {
    /// Reads at most `count` bytes
    /// from [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) instance
    /// and appends them to the ring buffer.
    /// If `count` is `None` then as much as possible bytes will be read.
    ///
    /// Returns `Ok(n)` if `read` succeeded. `n` is number of bytes been read.
    /// `n == 0` means that either `read` returned zero or ring buffer is full.
    ///
    /// If `read` is failed or returned an invalid number then error is returned.
    pub fn read_from(&mut self, reader: &mut dyn Read, count: Option<usize>) -> io::Result<usize> {
        let mut err = None;
        let n = unsafe {
            self.push_access(|left, _| -> usize {
                let left = match count {
                    Some(c) => {
                        if c < left.len() {
                            &mut left[0..c]
                        } else {
                            left
                        }
                    }
                    None => left,
                };
                match reader
                    .read(&mut *(left as *mut [MaybeUninit<u8>] as *mut [u8]))
                    .and_then(|n| {
                        if n <= left.len() {
                            Ok(n)
                        } else {
                            Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "Read operation returned an invalid number",
                            ))
                        }
                    }) {
                    Ok(n) => n,
                    Err(e) => {
                        err = Some(e);
                        0
                    }
                }
            })
        };
        match err {
            Some(e) => Err(e),
            None => Ok(n),
        }
    }
}

#[cfg(feature = "std")]
impl Write for Producer<u8> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let n = self.push_slice(buffer);
        if n == 0 && !buffer.is_empty() {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(n)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
*/
