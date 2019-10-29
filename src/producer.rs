use std::mem::{self, MaybeUninit};
use std::sync::{Arc, atomic::{Ordering}};
//use std::io::{self, Read, Write};

use crate::error::*;
use crate::ring_buffer::*;
//use crate::consumer::Consumer;


/// Producer part of ring buffer.
pub struct Producer<T> {
    pub(crate) rb: Arc<RingBuffer<T>>,
}


impl<T: Sized> Producer<T> {
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

    /// The length of the data in the buffer.
    pub fn len(&self) -> usize {
        self.rb.len()
    }

    /// The remaining space in the buffer.
    pub fn remaining(&self) -> usize {
        self.rb.remaining()
    }

    /// Appends an element to the ring buffer.
    /// On failure returns an error containing the element that hasn't beed appended.
    pub fn push(&mut self, elem: T) -> Result<(), PushError<T>> {
        let mut elem_opt = Some(elem);
        match unsafe { self.push_access(|slice, _| {
            mem::replace(slice.get_unchecked_mut(0), MaybeUninit::new(elem_opt.take().unwrap()));
            Ok((1, ()))
        }) } {
            Ok(res) => match res {
                Ok((n, ())) => {
                    debug_assert_eq!(n, 1);
                    Ok(())
                },
                Err(()) => unreachable!(),
            },
            Err(e) => match e {
                PushAccessError::Full => Err(PushError::Full(elem_opt.unwrap())),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}
/*
impl<T: Sized + Copy> Producer<T> {
    /// Appends elements from slice to the ring buffer.
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been appended to the ring buffer.
    pub fn push_slice(&mut self, elems: &[T]) -> Result<usize, PushSliceError> {
        let push_fn = |left: &mut [MaybeUninit<T>], right: &mut [MaybeUninit<T>]| {
            let write = |dst: &mut [MaybeUninit<T>], di: usize, src: &[T], si: usize| {
                unsafe { mem::replace(dst.get_unchecked_mut(di), MaybeUninit::new(*src.get_unchecked(si))); }
            };
            Ok((if elems.len() < left.len() {
                for i in 0..elems.len() {
                    write(left, i, elems, i);
                }
                elems.len()
            } else {
                for i in 0..left.len() {
                    write(left, i, elems, i);
                }
                if elems.len() < left.len() + right.len() {
                    for i in 0..(elems.len() - left.len()) {
                        write(right, i, elems, left.len() + i);
                    }
                    elems.len()
                } else {
                    for i in 0..right.len() {
                        write(right, i, elems, left.len() + i);
                    }
                    left.len() + right.len()
                }
            }, ()))
        };
        match unsafe { self.push_access(push_fn) } {
            Ok(res) => match res {
                Ok((n, ())) => {
                    Ok(n)
                },
                Err(()) => unreachable!(),
            },
            Err(e) => match e {
                PushAccessError::Full => Err(PushSliceError::Full),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}
*/
/*
impl<T: Sized> Producer<T> {
    /// Removes at most `count` elements from the `Consumer` of the ring buffer
    /// and appends them to the `Producer` of the another one.
    /// If `count` is `None` then as much as possible elements will be moved.
    ///
    /// Elements should be [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html).
    ///
    /// On success returns count of elements been moved.
    pub fn move_items(&mut self, other: &mut Consumer<T>, count: Option<usize>)
    -> Result<usize, MoveSliceError> {
        let move_fn = |left: &mut [MaybeUninit<T>], right: &mut [MaybeUninit<T>]|
        -> Result<(usize, ()), PopSliceError> {
            let (left, right) = match count {
                Some(c) => {
                    if c < left.len() {
                        (&mut left[0..c], &mut right[0..0])
                    } else if c < left.len() + right.len() {
                        let l = c - left.len();
                        (left, &mut right[0..l])
                    } else {
                        (left, right)
                    }
                },
                None => (left, right)
            };
            other.pop_slice(left).and_then(|n| {
                if n == left.len() {
                    other.pop_slice(right).and_then(|m| {
                        Ok((n + m, ()))
                    }).or_else(|e| {
                        match e {
                            PopSliceError::Empty => Ok((n, ())),
                        }
                    })
                } else {
                    debug_assert!(n < left.len());
                    Ok((n, ()))
                }
            })
        };
        match unsafe { self.push_access(move_fn) } {
            Ok(res) => match res {
                Ok((n, ())) => Ok(n),
                Err(e) => match e {
                    PopSliceError::Empty => Err(MoveSliceError::Empty),
                },
            },
            Err(e) => match e {
                PushAccessError::Full => Err(MoveSliceError::Full),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}
*/
/*
impl Producer<u8> {
    /// Reads at most `count` bytes
    /// from [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) instance
    /// and appends them to the ring buffer.
    /// If `count` is `None` then as much as possible bytes will be read.
    pub fn read_from(&mut self, reader: &mut dyn Read, count: Option<usize>)
    -> Result<usize, ReadFromError> {
        let push_fn = |left: &mut [u8], _right: &mut [u8]|
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
            reader.read(left).and_then(|n| {
                if n <= left.len() {
                    Ok((n, ()))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Read operation returned invalid number",
                    ))
                }
            })
        };
        match unsafe { self.push_access(push_fn) } {
            Ok(res) => match res {
                Ok((n, ())) => Ok(n),
                Err(e) => Err(ReadFromError::Read(e)),
            },
            Err(e) => match e {
                PushAccessError::Full => Err(ReadFromError::RbFull),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl Write for Producer<u8> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.push_slice(buffer).or_else(|e| match e {
            PushSliceError::Full => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Ring buffer is full",
            ))
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
*/
impl<T: Sized> Producer<T> {
    /// Allows to write into ring buffer memory directry.
    ///
    /// *This function is unsafe beacuse it gives access to possibly uninitialized memory
    /// and transfers to the user the responsibility of manually calling destructors*
    ///
    /// Takes a function `f` as argument.
    /// `f` takes two slices of ring buffer content (the second one may be empty). First slice contains older elements.
    ///
    /// `f` should return:
    /// + On success: pair of number of elements been written, and some arbitrary data.
    /// + On failure: some another arbitrary data.
    ///
    /// On success returns data returned from `f`.
    pub unsafe fn push_access<R, E, F>(&mut self, f: F) -> Result<Result<(usize, R), E>, PushAccessError>
    where R: Sized, E: Sized, F: FnOnce(&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) -> Result<(usize, R), E> {
        let head = self.rb.head.load(Ordering::Acquire);
        let tail = self.rb.tail.load(Ordering::Acquire);
        let len = self.rb.data.get_ref().len();

        let ranges = if tail >= head {
            if head > 0 {
                Ok((tail..len, 0..(head - 1)))
            } else {
                if tail < len - 1 {
                    Ok((tail..(len - 1), 0..0))
                } else {
                    Err(PushAccessError::Full)
                }
            }
        } else {
            if tail < head - 1 {
                Ok((tail..(head - 1), 0..0))
            } else {
                Err(PushAccessError::Full)
            }
        }?;

        let slices = (
            &mut self.rb.data.get_mut()[ranges.0],
            &mut self.rb.data.get_mut()[ranges.1],
        );

        match f(slices.0, slices.1) {
            Ok((n, r)) => {
                if n > slices.0.len() + slices.1.len() {
                    Err(PushAccessError::BadLen)
                } else {
                    let new_tail = (tail + n) % len;
                    self.rb.tail.store(new_tail, Ordering::Release);
                    Ok(Ok((n, r)))
                }
            },
            Err(e) => {
                Ok(Err(e))
            }
        }
    }
}
