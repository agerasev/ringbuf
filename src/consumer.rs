use std::mem::{self, MaybeUninit};
use std::sync::{Arc, atomic::{Ordering}};
//use std::io::{self, Read, Write};

use crate::error::*;
use crate::ring_buffer::*;
//use crate::producer::Producer;


/// Consumer part of ring buffer.
pub struct Consumer<T> {
    pub(crate) rb: Arc<RingBuffer<T>>,
}

impl<T: Sized> Consumer<T> {
    /// Returns capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        self.rb.capacity()
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.rb.is_empty()
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        self.rb.is_full()
    }

    /// The length of the data in the buffer
    pub fn len(&self) -> usize {
        self.rb.len()
    }

    /// The remaining space in the buffer.
    pub fn remaining(&self) -> usize {
        self.rb.remaining()
    }

    /// Removes first element from the ring buffer and returns it.
    pub fn pop(&mut self) -> Result<T, PopError> {
        match unsafe { self.pop_access(|slice, _| {
            let elem = mem::replace(slice.get_unchecked_mut(0), MaybeUninit::uninit()).assume_init();
            Ok((1, elem))
        }) } {
            Ok(res) => match res {
                Ok((n, elem)) => {
                    debug_assert_eq!(n, 1);
                    Ok(elem)
                },
                Err(()) => unreachable!(),
            },
            Err(e) => match e {
                PopAccessError::Empty => Err(PopError::Empty),
                PopAccessError::BadLen => unreachable!(),
            }
        }
    }
}

/*
impl<T: Sized + Copy> Consumer<T> {
    /// Removes first elements from the ring buffer and writes them into a slice.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been removed from the ring buffer.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> Result<usize, PopSliceError> {
        let pop_fn = |left: &mut [T], right: &mut [T]| {
            let elems_len = elems.len();
            Ok((if elems_len < left.len() {
                elems.copy_from_slice(&left[0..elems_len]);
                elems_len
            } else {
                elems[0..left.len()].copy_from_slice(left);
                if elems_len < left.len() + right.len() {
                    elems[left.len()..elems_len]
                        .copy_from_slice(&right[0..(elems_len - left.len())]);
                    elems_len
                } else {
                    elems[left.len()..(left.len() + right.len())].copy_from_slice(right);
                    left.len() + right.len()
                }
            }, ()))
        };
        match unsafe { self.pop_access(pop_fn) } {
            Ok(res) => match res {
                Ok((n, ())) => {
                    Ok(n)
                },
                Err(()) => unreachable!(),
            },
            Err(e) => match e {
                PopAccessError::Empty => Err(PopSliceError::Empty),
                PopAccessError::BadLen => unreachable!(),
            }
        }
    }

    /// Removes at most `count` elements from the `Consumer` of the ring buffer
    /// and appends them to the `Producer` of the another one.
    /// If `count` is `None` then as much as possible elements will be moved.
    ///
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been moved.
    pub fn move_slice(&mut self, other: &mut Producer<T>, count: Option<usize>)
    -> Result<usize, MoveSliceError> {
        other.move_slice(self, count)
    }
}
*/
/*
impl Consumer<u8> {
    /// Removes at most first `count` bytes from the ring buffer and writes them into
    /// a [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html) instance.
    /// If `count` is `None` then as much as possible bytes will be written.
    pub fn write_into(&mut self, writer: &mut dyn Write, count: Option<usize>)
    -> Result<usize, WriteIntoError> {
        let pop_fn = |left: &mut [u8], _right: &mut [u8]|
        -> Result<(usize, ()), io::Error> {
            let left = match count {
                Some(c) => {
                    if c < left.len() {
                        &mut left[0..c]
                    } else {
                        left
                    }
                },
                None => left,
            };
            writer.write(left).and_then(|n| {
                if n <= left.len() {
                    Ok((n, ()))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Write operation returned invalid number",
                    ))
                }
            })
        };
        match unsafe { self.pop_access(pop_fn) } {
            Ok(res) => match res {
                Ok((n, ())) => Ok(n),
                Err(e) => Err(WriteIntoError::Write(e)),
            },
            Err(e) => match e {
                PopAccessError::Empty => Err(WriteIntoError::RbEmpty),
                PopAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl Read for Consumer<u8> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.pop_slice(buffer).or_else(|e| match e {
            PopSliceError::Empty => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Ring buffer is empty",
            ))
        })
    }
}
*/

impl<T: Sized> Consumer<T> {
    /// Allows to read from ring buffer memory directry.
    ///
    /// *This function is unsafe beacuse it gives access to possibly uninitialized memory
    /// and transfers to the user the responsibility of manually calling destructors*
    ///
    /// Takes a function `f` as argument.
    /// `f` takes two slices of ring buffer content (the second one may be empty). First slice contains older elements.
    ///
    /// `f` should return:
    /// + On success: pair of number of elements been read, and some arbitrary data.
    /// + On failure: some another arbitrary data.
    ///
    /// On success returns data returned from `f`.
    pub unsafe fn pop_access<R, E, F>(&mut self, f: F) -> Result<Result<(usize, R), E>, PopAccessError>
    where R: Sized, E: Sized, F: FnOnce(&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) -> Result<(usize, R), E> {
        let head = self.rb.head.load(Ordering::Acquire);
        let tail = self.rb.tail.load(Ordering::Acquire);
        let len = self.rb.data.get_ref().len();

        let ranges = if head < tail {
            Ok((head..tail, 0..0))
        } else if head > tail {
            Ok((head..len, 0..tail))
        } else {
            Err(PopAccessError::Empty)
        }?;

        let slices = (
            &mut self.rb.data.get_mut()[ranges.0],
            &mut self.rb.data.get_mut()[ranges.1],
        );

        match f(slices.0, slices.1) {
            Ok((n, r)) => {
                if n > slices.0.len() + slices.1.len() {
                    Err(PopAccessError::BadLen)
                } else {
                    let new_head = (head + n) % len;
                    self.rb.head.store(new_head, Ordering::Release);
                    Ok(Ok((n, r)))
                }
            },
            Err(e) => {
                Ok(Err(e))
            }
        }
    }
}
