//! Lock-free single-producer single-consumer ring buffer
//!
//! # Examples
//!
//! ## Simple example
//!
//! ```rust
//! # extern crate ringbuf;
//! use ringbuf::{RingBuffer, PushError, PopError};
//! # fn main() {
//! let rb = RingBuffer::<i32>::new(2);
//! let (mut prod, mut cons) = rb.split();
//! 
//! prod.push(0).unwrap();
//! prod.push(1).unwrap();
//! assert_eq!(prod.push(2), Err((PushError::Full, 2)));
//! 
//! assert_eq!(cons.pop().unwrap(), 0);
//! 
//! prod.push(2).unwrap();
//! 
//! assert_eq!(cons.pop().unwrap(), 1);
//! assert_eq!(cons.pop().unwrap(), 2);
//! assert_eq!(cons.pop(), Err(PopError::Empty));
//! # }
//! ```
//!
//! ## Message transfer
//!
//! This is more complicated example of transfering text message between threads.
//!
//! ```rust
//! # extern crate ringbuf;
//! use std::io::{self, Read};
//! use std::thread;
//! use std::time::{Duration};
//! 
//! use ringbuf::{RingBuffer, ReadFrom, WriteInto};
//! 
//! # fn main() {
//! let rb = RingBuffer::<u8>::new(10);
//! let (mut prod, mut cons) = rb.split();
//! 
//! let smsg = "The quick brown fox jumps over the lazy dog";
//! 
//! let pjh = thread::spawn(move || {
//!     println!("-> sending message: '{}'", smsg);
//! 
//!     let zero = [0 as u8];
//!     let mut bytes = smsg.as_bytes().chain(&zero[..]);
//!     loop {
//!         match prod.read_from(&mut bytes) {
//!             Ok(n) => {
//!                 if n == 0 {
//!                     break;
//!                 }
//!                 println!("-> {} bytes sent", n);
//!             },
//!             Err(err) => {
//!                 assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
//!                 println!("-> buffer is full, waiting");
//!                 thread::sleep(Duration::from_millis(1));
//!             },
//!         }
//!     }
//! 
//!     println!("-> message sent");
//! });
//! 
//! let cjh = thread::spawn(move || {
//!     println!("<- receiving message");
//! 
//!     let mut bytes = Vec::<u8>::new();
//!     loop {
//!         match cons.write_into(&mut bytes) {
//!             Ok(n) => println!("<- {} bytes received", n),
//!             Err(err) => {
//!                 assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
//!                 if bytes.ends_with(&[0]) {
//!                     break;
//!                 } else {
//!                     println!("<- buffer is empty, waiting");
//!                     thread::sleep(Duration::from_millis(1));
//!                 }
//!             },
//!         }
//!     }
//! 
//!     assert_eq!(bytes.pop().unwrap(), 0);
//!     let msg = String::from_utf8(bytes).unwrap();
//!     println!("<- message received: '{}'", msg);
//! 
//!     msg
//! });
//! 
//! pjh.join().unwrap();
//! let rmsg = cjh.join().unwrap();
//! 
//! assert_eq!(smsg, rmsg);
//! # }
//! ```
//!


#![cfg_attr(rustc_nightly, feature(test))]

#[cfg(test)]
#[cfg(rustc_nightly)]
extern crate test;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[cfg(rustc_nightly)]
mod benchmarks;


use std::mem;
use std::cell::{UnsafeCell};
use std::sync::{Arc, atomic::{Ordering, AtomicUsize}};
use std::io::{self, Read, Write};


#[derive(Debug, PartialEq, Eq)]
pub enum PushAccessError {
    /// Cannot push: ring buffer is full.
    Full,
    /// User function returned invalid length.
    BadLen,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PopAccessError {
    /// Cannot pop: ring buffer is empty.
    Empty,
    /// User function returned invalid length.
    BadLen,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PushError {
    /// Cannot push: ring buffer is full.
    Full,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PopError {
    /// Cannot pop: ring buffer is empty.
    Empty,
}

struct SharedVec<T: Sized> {
    cell: UnsafeCell<Vec<T>>,
}

unsafe impl<T: Sized> Sync for SharedVec<T> {}

impl<T: Sized> SharedVec<T> {
    fn new(data: Vec<T>) -> Self {
        Self { cell: UnsafeCell::new(data) }
    }
    unsafe fn get_ref(&self) -> &Vec<T> {
        self.cell.get().as_ref().unwrap()
    }
    unsafe fn get_mut(&self) -> &mut Vec<T> {
        self.cell.get().as_mut().unwrap()
    }
}

/// Ring buffer itself.
pub struct RingBuffer<T: Sized> {
    data: SharedVec<T>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

/// Producer part of ring buffer.
pub struct Producer<T> {
    rb: Arc<RingBuffer<T>>,
}

/// Consumer part of ring buffer.
pub struct Consumer<T> {
    rb: Arc<RingBuffer<T>>,
}

impl<T: Sized> RingBuffer<T> {
    /// Creates new instance of a ring buffer.
    pub fn new(capacity: usize) -> Self {
        let vec_cap = capacity + 1;
        let mut data = Vec::with_capacity(vec_cap);
        unsafe { data.set_len(vec_cap); }
        Self {
            data: SharedVec::new(data),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Splits ring buffer into producer and consumer.
    pub fn split(self) -> (Producer<T>, Consumer<T>) {
        let arc = Arc::new(self);
        (
            Producer { rb: arc.clone() },
            Consumer { rb: arc },
        )
    }

    /// Returns capacity of a ring buffer.
    pub fn capacity(&self) -> usize {
        unsafe { self.data.get_ref() }.len() - 1
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        head == tail
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        (tail + 1) % (self.capacity() + 1) == head
    }
}

impl<T: Sized> Drop for RingBuffer<T> {
    fn drop(&mut self) {
        let data = unsafe { self.data.get_mut() };

        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        let len = data.len();
        
        let slices = if head <= tail {
            (head..tail, 0..0)
        } else {
            (head..len, 0..tail)
        };

        let drop = |elem_ref: &mut T| {
            mem::drop(mem::replace(elem_ref, unsafe { mem::uninitialized() }));
        };
        for elem in data[slices.0].iter_mut() {
            drop(elem);
        }
        for elem in data[slices.1].iter_mut() {
            drop(elem);
        }

        unsafe { data.set_len(0); }
    }
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
    
    /// Pushes element into ring buffer.
    ///
    /// On error returns pair of error and element that wasn't pushed.
    pub fn push(&mut self, elem: T) -> Result<(), (PushError, T)> {
        let mut elem_opt = Some(elem);
        match unsafe { self.push_access(|slice, _| {
            mem::forget(mem::replace(&mut slice[0], elem_opt.take().unwrap()));
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
                PushAccessError::Full => Err((PushError::Full, elem_opt.unwrap())),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl<T: Sized + Copy> Producer<T> {
    /// Pushes elements from slice into ring buffer. Elements should be be cloneable.
    ///
    /// On success returns count of elements been pushed into ring buffer.
    pub fn push_slice(&mut self, elems: &[T]) -> Result<usize, PushError> {
        let push_fn = |left: &mut [T], right: &mut [T]| {
            Ok((if elems.len() < left.len() {
                left[0..elems.len()].copy_from_slice(elems);
                elems.len()
            } else {
                left.copy_from_slice(&elems[0..left.len()]);
                if elems.len() < left.len() + right.len() {
                    right[0..(elems.len() - left.len())]
                        .copy_from_slice(&elems[left.len()..elems.len()]);
                    elems.len()
                } else {
                    right.copy_from_slice(&elems[left.len()..(left.len() + right.len())]);
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
                PushAccessError::Full => Err(PushError::Full),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl<T: Sized> Producer<T> {
    /// Allows to write into ring buffer memory directry.
    ///
    /// *This function is unsafe beacuse it gives access to possibly uninitialized memory
    /// and transfers responsibility of manually calling destructors*
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
    where R: Sized, E: Sized, F: FnOnce(&mut [T], &mut [T]) -> Result<(usize, R), E> {
        let head = self.rb.head.load(Ordering::SeqCst);
        let tail = self.rb.tail.load(Ordering::SeqCst);
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
                    self.rb.tail.store(new_tail, Ordering::SeqCst);
                    Ok(Ok((n, r)))
                }
            },
            Err(e) => {
                Ok(Err(e))
            }
        }
    }
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

    /// Retrieves element from ring buffer.
    ///
    /// On success returns element been retrieved.
    pub fn pop(&mut self) -> Result<T, PopError> {
        match unsafe { self.pop_access(|slice, _| {
            let elem = mem::replace(&mut slice[0], mem::uninitialized());
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

impl<T: Sized + Copy> Consumer<T> {
    /// Retrieves elements from ring buffer into slice. Elements should be cloneable.
    ///
    /// On success returns count of elements been retrieved from ring buffer.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> Result<usize, PopError> {
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
                PopAccessError::Empty => Err(PopError::Empty),
                PopAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl<T: Sized> Consumer<T> {
    /// Allows to read from ring buffer memory directry.
    ///
    /// *This function is unsafe beacuse it gives access to possibly uninitialized memory
    /// and transfers responsibility of manually calling destructors*
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
    where R: Sized, E: Sized, F: FnOnce(&mut [T], &mut [T]) -> Result<(usize, R), E> {
        let head = self.rb.head.load(Ordering::SeqCst);
        let tail = self.rb.tail.load(Ordering::SeqCst);
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
                    self.rb.head.store(new_head, Ordering::SeqCst);
                    Ok(Ok((n, r)))
                }
            },
            Err(e) => {
                Ok(Err(e))
            }
        }
    }
}

/// Something that can read from [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) instance.
pub trait ReadFrom {
    /// Read from [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) instance.
    fn read_from(&mut self, reader: &mut dyn Read) -> io::Result<usize>;
}

/// Something that can write into [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html) instance.
pub trait WriteInto {
    /// Write into [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html) instance.
    fn write_into(&mut self, writer: &mut dyn Write) -> io::Result<usize>;
}

impl ReadFrom for Producer<u8> {
    fn read_from(&mut self, reader: &mut dyn Read) -> io::Result<usize> {
        let push_fn = |left: &mut [u8], _right: &mut [u8]| -> io::Result<(usize, ())> {
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
                Err(e) => Err(e),
            },
            Err(e) => match e {
                PushAccessError::Full => Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Ring buffer is full",
                )),
                PushAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl WriteInto for Consumer<u8> {
    fn write_into(&mut self, writer: &mut dyn Write) -> io::Result<usize> {
        let pop_fn = |left: &mut [u8], _right: &mut [u8]| -> io::Result<(usize, ())> {
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
                Err(e) => Err(e),
            },
            Err(e) => match e {
                PopAccessError::Empty => Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Ring buffer is empty",
                )),
                PopAccessError::BadLen => unreachable!(),
            }
        }
    }
}

impl Write for Producer<u8> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.push_slice(buffer).or_else(|e| match e {
            PushError::Full => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Ring buffer is full",
            ))
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Consumer<u8> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.pop_slice(buffer).or_else(|e| match e {
            PopError::Empty => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Ring buffer is empty",
            ))
        })
    }
}


#[cfg(test)]
#[test]
fn dummy_test() {
    let rb = RingBuffer::<i32>::new(16);
    rb.split();
}
