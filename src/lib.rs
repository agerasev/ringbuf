use std::mem;
use std::cell::{UnsafeCell};
use std::sync::{Arc, atomic::{Ordering, AtomicUsize}};
//use std::io::{self, Read, Write};


pub enum PushAccessError {
    Full,
    BadLen,
}

pub enum PopAccessError {
    Empty,
    BadLen,
}

pub enum PushError {
    Full,
}

pub enum PopError {
    Empty,
}

pub struct RingBuffer<T: Sized> {
    data: UnsafeCell<Vec<T>>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

pub struct Producer<T> {
    rb: Arc<RingBuffer<T>>,
}

pub struct Consumer<T> {
    rb: Arc<RingBuffer<T>>,
}

impl<T: Sized + Copy + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: UnsafeCell::new((0..capacity).map(|_| { T::default() }).collect()),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }
}

impl<T: Sized + Copy> RingBuffer<T> {
    pub unsafe fn new_uninitialized(capacity: usize) -> Self {
        Self {
            data: UnsafeCell::new((0..capacity).map(|_| { mem::uninitialized() }).collect()),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn split(self) -> (Producer<T>, Consumer<T>) {
        let arc = Arc::new(self);
        (
            Producer { rb: arc.clone() },
            Consumer { rb: arc },
        )
    }
}

impl<T: Sized + Copy> Producer<T> {
    pub fn push_access<R, E, F>(&self, f: F) -> Result<Result<(usize, R), E>, PushAccessError>
    where R: Sized, E: Sized, F: FnOnce(&mut [T], &mut [T]) -> Result<(usize, R), E> {
        let vptr = self.rb.data.get();

        let head = self.rb.head.load(Ordering::SeqCst);
        let tail = self.rb.tail.load(Ordering::SeqCst);
        let len = unsafe { vptr.as_ref() }.unwrap().len();

        let ranges = if tail >= head {
            if head > 0 {
                Ok((tail..len, 0..(head - 1)))
            } else {
                if tail <= len - 1 {
                    Ok((tail..len, 0..0))
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
            &mut unsafe { vptr.as_mut() }.unwrap()[ranges.0],
            &mut unsafe { vptr.as_mut() }.unwrap()[ranges.1],
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

    pub fn push_many(&self, elems: &[T]) -> Result<usize, PushError> {
        match self.push_access(|left, right| {
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
                    right.copy_from_slice(&elems[left.len()..elems.len()]);
                    left.len() + right.len()
                }
            }, ()))
        }) {
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

    pub fn push(&self, elem: T) -> Option<()> {
        match self.push_many(&[elem]) {
            Ok(n) => {
                debug_assert_eq!(n, 1);
                Some(())
            },
            Err(PushError::Full) => None,
        }
    }
}

impl<T: Sized + Copy> Consumer<T> {
    pub fn pop_access<R, E, F>(&self, f: F) -> Result<Result<(usize, R), E>, PopAccessError>
    where R: Sized, E: Sized, F: FnOnce(&[T], &[T]) -> Result<(usize, R), E> {
        let data = unsafe { self.rb.data.get().as_ref() }.unwrap();

        let head = self.rb.head.load(Ordering::SeqCst);
        let tail = self.rb.tail.load(Ordering::SeqCst);
        let len = data.len();

        let ranges = if head < tail {
            Ok((head..tail, 0..0))
        } else if head > tail {
            Ok((head..len, 0..tail))
        } else {
            Err(PopAccessError::Empty)
        }?;

        let slices = (&data[ranges.0], &data[ranges.1]);

        match f(slices.0, slices.1) {
            Ok((n, r)) => {
                if n > slices.0.len() + slices.1.len() {
                    Err(PopAccessError::BadLen)
                } else {
                    let new_head = (tail + n) % len;
                    self.rb.head.store(new_head, Ordering::SeqCst);
                    Ok(Ok((n, r)))
                }
            },
            Err(e) => {
                Ok(Err(e))
            }
        }
    }

    pub fn pop_many(&self, elems: &mut [T]) -> Result<usize, PopError> {
        match self.pop_access(|left, right| {
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
                    elems[left.len()..elems_len].copy_from_slice(right);
                    left.len() + right.len()
                }
            }, ()))
        }) {
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

    pub fn pop(&self) -> Option<T> {
        let mut arr: [T; 1] = unsafe { mem::uninitialized() };
        match self.pop_many(&mut arr) {
            Ok(n) => {
                debug_assert_eq!(n, 1);
                Some(arr[0])
            },
            Err(PopError::Empty) => None,
        }
    }
}

/*
pub trait WriteAccess {
    fn write_access<F>(n: usize, f: F) -> io::Result<usize>
    where F: Fn(&mut [u8]) -> io::Result<usize>;
}

pub trait ReadAccess {
    fn read_access<F>(n: usize, f: F) -> io::Result<usize>
    where F: Fn(&[u8]) -> io::Result<usize>;
}

impl WriteAccess for Producer<u8> {
    fn write_access<F>(n: usize, f: F) -> io::Result<usize>
    where F: Fn(&mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl ReadAccess for Consumer<u8> {
    fn read_access<F>(n: usize, f: F) -> io::Result<usize>
    where F: Fn(&[u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl Write for Producer<u8> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Consumer<u8> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}
*/

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
