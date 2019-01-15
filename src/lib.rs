#![cfg_attr(rustc_nightly, feature(test))]

use std::mem;
use std::cell::{UnsafeCell};
use std::sync::{Arc, atomic::{Ordering, AtomicUsize}};
//use std::io::{self, Read, Write};

/// `Producer::push_access` error info. 
#[derive(Debug, PartialEq, Eq)]
pub enum PushAccessError {
    /// Cannot push - `RingBuffer` is full.
    Full,
    /// User function returned invalid length.
    BadLen,
}

/// `Receiver::pop_access` error info.
#[derive(Debug, PartialEq, Eq)]
pub enum PopAccessError {
    /// Cannot pop - `RingBuffer` is empty.
    Empty,
    /// User function returned invalid length.
    BadLen,
}

/// `Producer::push` and `Producer::push_slice` error info. 
#[derive(Debug, PartialEq, Eq)]
pub enum PushError {
    /// Cannot push - `RingBuffer` is full.
    Full,
}

/// `Receiver::pop` and `Receiver::pop_slice` error info.
#[derive(Debug, PartialEq, Eq)]
pub enum PopError {
    /// Cannot pop - `RingBuffer` is empty.
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

/// Producer part of `RingBuffer`.
pub struct Producer<T> {
    rb: Arc<RingBuffer<T>>,
}

/// Consumer part of `RingBuffer`.
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
    /// Pushes element into `RingBuffer`.
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

    /// Returns capacity of the `RingBuffer`.
    pub fn capacity(&self) -> usize {
        self.rb.capacity()
    }

    /// Checks if the `RingBuffer` is full.
    pub fn is_full(&self) -> bool {
        let head = self.rb.head.load(Ordering::SeqCst);
        let tail = self.rb.tail.load(Ordering::SeqCst);
        (tail + 1) % (self.capacity() + 1) == head
    }
}

impl<T: Sized + Clone> Producer<T> {
    /// Pushes elements from slice into `RingBuffer`. Elements should be be cloneable.
    ///
    /// On success returns count of elements been pushed into `RingBuffer`.
    pub fn push_slice(&mut self, elems: &[T]) -> Result<usize, PushError> {
        let push_fn = |left: &mut [T], right: &mut [T]| {
            Ok((if elems.len() < left.len() {
                left[0..elems.len()].clone_from_slice(elems);
                elems.len()
            } else {
                left.clone_from_slice(&elems[0..left.len()]);
                if elems.len() < left.len() + right.len() {
                    right[0..(elems.len() - left.len())]
                        .clone_from_slice(&elems[left.len()..elems.len()]);
                    elems.len()
                } else {
                    right.clone_from_slice(&elems[left.len()..(left.len() + right.len())]);
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
    /// Allows to write into `RingBuffer` memory directry.
    ///
    /// *This function is unsafe beacuse it gives access to possibly uninitialized memory.*
    ///
    /// Takes a function `f` as argument.
    /// `f` takes two slices of `RingBuffer` content (the second one may be empty). First slice contains older elements.
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
    /// Retrieves element from `RingBuffer`.
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

    /// Returns capacity of the `RingBuffer`.
    pub fn capacity(&self) -> usize {
        self.rb.capacity()
    }

    /// Checks if the `RingBuffer` is empty.
    pub fn is_empty(&self) -> bool {
        let head = self.rb.head.load(Ordering::SeqCst);
        let tail = self.rb.tail.load(Ordering::SeqCst);
        head == tail
    }
}

impl<T: Sized + Clone> Consumer<T> {
    /// Retrieves elements from `RingBuffer` into slice. Elements should be cloneable.
    ///
    /// On success returns count of elements been retrieved from `RingBuffer`.
    pub fn pop_slice(&mut self, elems: &mut [T]) -> Result<usize, PopError> {
        let pop_fn = |left: &mut [T], right: &mut [T]| {
            let elems_len = elems.len();
            Ok((if elems_len < left.len() {
                elems.clone_from_slice(&left[0..elems_len]);
                elems_len
            } else {
                elems[0..left.len()].clone_from_slice(left);
                if elems_len < left.len() + right.len() {
                    elems[left.len()..elems_len]
                        .clone_from_slice(&right[0..(elems_len - left.len())]);
                    elems_len
                } else {
                    elems[left.len()..(left.len() + right.len())].clone_from_slice(right);
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
    /// Allows to read from `RingBuffer` memory directry.
    ///
    /// *This function is unsafe beacuse it gives access to possibly uninitialized memory.*
    ///
    /// Takes a function `f` as argument.
    /// `f` takes two slices of `RingBuffer` content (the second one may be empty). First slice contains older elements.
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
#[macro_use]
extern crate matches;

#[cfg(test)]
mod tests {

    use super::*;

    use std::cell::{Cell};
    use std::thread;
    use std::io::{Read, Write};
    use std::time::{Duration};

    fn head_tail<T>(rb: &RingBuffer<T>) -> (usize, usize) {
        (rb.head.load(Ordering::SeqCst), rb.tail.load(Ordering::SeqCst))
    }

    #[test]
    fn capacity() {
        let cap = 13;
        let buf = RingBuffer::<i32>::new(cap);
        assert_eq!(buf.capacity(), cap);
    }

    #[test]
    fn split_capacity() {
        let cap = 13;
        let buf = RingBuffer::<i32>::new(cap);
        let (prod, cons) = buf.split();
        
        assert_eq!(prod.capacity(), cap);
        assert_eq!(cons.capacity(), cap);
    }

    #[test]
    fn split_threads() {
        let buf = RingBuffer::<i32>::new(10);
        let (prod, cons) = buf.split();
        
        let pjh = thread::spawn(move || {
            let _ = prod;
        });

        let cjh = thread::spawn(move || {
            let _ = cons;
        });

        pjh.join().unwrap();
        cjh.join().unwrap();
    }

    #[test]
    fn push() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, _) = buf.split();
        

        assert_eq!(head_tail(&prod.rb), (0, 0));

        assert_matches!(prod.push(123), Ok(()));
        assert_eq!(head_tail(&prod.rb), (0, 1));

        assert_matches!(prod.push(234), Ok(()));
        assert_eq!(head_tail(&prod.rb), (0, 2));

        assert_matches!(prod.push(345), Err((PushError::Full, 345)));
        assert_eq!(head_tail(&prod.rb), (0, 2));
    }

    #[test]
    fn pop_empty() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (_, mut cons) = buf.split();


        assert_eq!(head_tail(&cons.rb), (0, 0));

        assert_eq!(cons.pop(), Err(PopError::Empty));
        assert_eq!(head_tail(&cons.rb), (0, 0));
    }

    #[test]
    fn push_pop_one() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, mut cons) = buf.split();

        let vcap = cap + 1;
        let values = [12, 34, 56, 78, 90];
        assert_eq!(head_tail(&cons.rb), (0, 0));

        for (i, v) in values.iter().enumerate() {
            assert_matches!(prod.push(*v), Ok(()));
            assert_eq!(head_tail(&cons.rb), (i % vcap, (i + 1) % vcap));

            match cons.pop() {
                Ok(w) => assert_eq!(w, *v),
                other => panic!(other),
            }
            assert_eq!(head_tail(&cons.rb), ((i + 1) % vcap, (i + 1) % vcap));

            assert_eq!(cons.pop(), Err(PopError::Empty));
            assert_eq!(head_tail(&cons.rb), ((i + 1) % vcap, (i + 1) % vcap));
        }
    }

    #[test]
    fn push_pop_all() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, mut cons) = buf.split();

        let vcap = cap + 1;
        let values = [(12, 34, 13), (56, 78, 57), (90, 10, 91)];
        assert_eq!(head_tail(&cons.rb), (0, 0));

        for (i, v) in values.iter().enumerate() {
            assert_matches!(prod.push(v.0), Ok(()));
            assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 1) % vcap));

            assert_matches!(prod.push(v.1), Ok(()));
            assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 2) % vcap));

            match prod.push(v.2) {
                Err((PushError::Full, w)) => assert_eq!(w, v.2),
                other => panic!(other),
            }
            assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 2) % vcap));


            match cons.pop() {
                Ok(w) => assert_eq!(w, v.0),
                other => panic!(other),
            }
            assert_eq!(head_tail(&cons.rb), ((cap*i + 1) % vcap, (cap*i + 2) % vcap));

            match cons.pop() {
                Ok(w) => assert_eq!(w, v.1),
                other => panic!(other),
            }
            assert_eq!(head_tail(&cons.rb), ((cap*i + 2) % vcap, (cap*i + 2) % vcap));

            assert_eq!(cons.pop(), Err(PopError::Empty));
            assert_eq!(head_tail(&cons.rb), ((cap*i + 2) % vcap, (cap*i + 2) % vcap));
        }
    }

    #[test]
    fn producer_full() {
        let buf = RingBuffer::<i32>::new(1);
        let (mut prod, _) = buf.split();

        assert!(!prod.is_full());

        assert_matches!(prod.push(123), Ok(()));
        assert!(prod.is_full());
    }

    #[test]
    fn consumer_empty() {
        let buf = RingBuffer::<i32>::new(1);
        let (mut prod, cons) = buf.split();


        assert_eq!(head_tail(&cons.rb), (0, 0));
        assert!(cons.is_empty());

        assert_matches!(prod.push(123), Ok(()));
        assert!(!cons.is_empty());
    }

    #[derive(Debug)]
    struct Dropper<'a> {
        cnt: &'a Cell<i32>,
    }

    impl<'a> Dropper<'a> {
        fn new(c: &'a Cell<i32>) -> Self {
            Self { cnt: c }
        }
    }

    impl<'a> Drop for Dropper<'a> {
        fn drop(&mut self) {
            self.cnt.set(self.cnt.get() + 1);
        }
    }

    #[test]
    fn drop() {
        let (ca, cb) = (Cell::new(0), Cell::new(0));
        let (da, db) = (Dropper::new(&ca), Dropper::new(&cb));

        let cap = 3;
        let buf = RingBuffer::new(cap);

        {
            let (mut prod, mut cons) = buf.split();

            assert_eq!((ca.get(), cb.get()), (0, 0));

            prod.push(da).unwrap();
            assert_eq!((ca.get(), cb.get()), (0, 0));

            prod.push(db).unwrap();
            assert_eq!((ca.get(), cb.get()), (0, 0));

            cons.pop().unwrap();
            assert_eq!((ca.get(), cb.get()), (1, 0));
        }
        
        assert_eq!((ca.get(), cb.get()), (1, 1));
    }

    #[test]
    fn push_access() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, mut cons) = buf.split();

        let vs_20 = (123, 456);
        let push_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            left[0] = vs_20.0;
            left[1] = vs_20.1;
            Ok((2, ()))
        };

        assert_eq!(unsafe { prod.push_access(push_fn_20) }.unwrap().unwrap(), (2, ()));

        assert_eq!(cons.pop().unwrap(), vs_20.0);
        assert_eq!(cons.pop().unwrap(), vs_20.1);
        assert_matches!(cons.pop(), Err(PopError::Empty));

        let vs_11 = (123, 456);
        let push_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 1);
            left[0] = vs_11.0;
            right[0] = vs_11.1;
            Ok((2, ()))
        };

        assert_eq!(unsafe { prod.push_access(push_fn_11) }.unwrap().unwrap(), (2, ()));

        assert_eq!(cons.pop().unwrap(), vs_11.0);
        assert_eq!(cons.pop().unwrap(), vs_11.1);
        assert_matches!(cons.pop(), Err(PopError::Empty));
    }

    /*
    /// This test doesn't compile.
    /// And that's good :)
    #[test]
    fn push_access_oref() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, _) = buf.split();

        let mut ovar = 123;
        let mut oref = &mut 123;
        let push_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            left[0] = 456;
            oref = &mut left[0];
            Ok((1, ()))
        };

        assert_eq!(unsafe {
            prod.push_access(push_fn_20)
        }.unwrap().unwrap(), (1, ()));

        assert_eq!(*oref, 456);
    }
    */

    #[test]
    fn pop_access_full() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (_, mut cons) = buf.split();

        let dummy_fn = |_l: &mut [i32], _r: &mut [i32]| -> Result<(usize, ()), ()> {
            if true {
                Ok((0, ()))
            } else {
                Err(())
            }
        };
        assert_matches!(unsafe { cons.pop_access(dummy_fn) }, Err(PopAccessError::Empty));
    }

    #[test]
    fn pop_access_empty() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (_, mut cons) = buf.split();

        let dummy_fn = |_l: &mut [i32], _r: &mut [i32]| -> Result<(usize, ()), ()> {
            if true {
                Ok((0, ()))
            } else {
                Err(())
            }
        };
        assert_matches!(unsafe { cons.pop_access(dummy_fn) }, Err(PopAccessError::Empty));
    }

    #[test]
    fn pop_access() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, mut cons) = buf.split();


        let vs_20 = (123, 456);

        assert_matches!(prod.push(vs_20.0), Ok(()));
        assert_matches!(prod.push(vs_20.1), Ok(()));
        assert_matches!(prod.push(0), Err((PushError::Full, 0)));

        let pop_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], vs_20.0);
            assert_eq!(left[1], vs_20.1);
            Ok((2, ()))
        };

        assert_eq!(unsafe { cons.pop_access(pop_fn_20) }.unwrap().unwrap(), (2, ()));


        let vs_11 = (123, 456);
        
        assert_matches!(prod.push(vs_11.0), Ok(()));
        assert_matches!(prod.push(vs_11.1), Ok(()));
        assert_matches!(prod.push(0), Err((PushError::Full, 0)));
        
        let pop_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 1);
            assert_eq!(left[0], vs_11.0);
            assert_eq!(right[0], vs_11.1);
            Ok((2, ()))
        };

        assert_eq!(unsafe { cons.pop_access(pop_fn_11) }.unwrap().unwrap(), (2, ()));

    }

    #[test]
    fn push_access_return() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, mut cons) = buf.split();

        let push_fn_3 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            Ok((3, ()))
        };

        assert_matches!(unsafe { prod.push_access(push_fn_3) }, Err(PushAccessError::BadLen)
        );

        let push_fn_err = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), i32> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            Err(123)
        };

        assert_matches!(unsafe { prod.push_access(push_fn_err) }, Ok(Err(123))
        );

        let push_fn_0 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            Ok((0, ()))
        };

        assert_matches!(unsafe { prod.push_access(push_fn_0) }, Ok(Ok((0, ())))
        );

        let push_fn_1 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            left[0] = 12;
            Ok((1, ()))
        };

        assert_matches!(unsafe { prod.push_access(push_fn_1) }, Ok(Ok((1, ())))
        );

        let push_fn_2 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 0);
            left[0] = 34;
            Ok((1, ()))
        };

        assert_matches!(unsafe { prod.push_access(push_fn_2) }, Ok(Ok((1, ())))
        );

        assert_eq!(cons.pop().unwrap(), 12);
        assert_eq!(cons.pop().unwrap(), 34);
        assert_matches!(cons.pop(), Err(PopError::Empty));
    }

    #[test]
    fn pop_access_return() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, mut cons) = buf.split();

        assert_matches!(prod.push(12), Ok(()));
        assert_matches!(prod.push(34), Ok(()));
        assert_matches!(prod.push(0), Err((PushError::Full, 0)));

        let pop_fn_3 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            Ok((3, ()))
        };

        assert_matches!(unsafe { cons.pop_access(pop_fn_3) }, Err(PopAccessError::BadLen)
        );

        let pop_fn_err = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), i32> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            Err(123)
        };

        assert_matches!(unsafe { cons.pop_access(pop_fn_err) }, Ok(Err(123))
        );

        let pop_fn_0 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            Ok((0, ()))
        };

        assert_matches!(unsafe { cons.pop_access(pop_fn_0) }, Ok(Ok((0, ())))
        );

        let pop_fn_1 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], 12);
            Ok((1, ()))
        };

        assert_matches!(unsafe { cons.pop_access(pop_fn_1) }, Ok(Ok((1, ())))
        );

        let pop_fn_2 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], 34);
            Ok((1, ()))
        };

        assert_matches!(unsafe { cons.pop_access(pop_fn_2) }, Ok(Ok((1, ())))
        );
    }

    #[test]
    fn push_pop_access() {
        let cap = 2;
        let buf = RingBuffer::<i32>::new(cap);
        let (mut prod, mut cons) = buf.split();

        let vs_20 = (123, 456);
        let push_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            left[0] = vs_20.0;
            left[1] = vs_20.1;
            Ok((2, ()))
        };

        assert_eq!(unsafe { prod.push_access(push_fn_20) }.unwrap().unwrap(), (2, ()));

        let pop_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 0);
            assert_eq!(left[0], vs_20.0);
            assert_eq!(left[1], vs_20.1);
            Ok((2, ()))
        };

        assert_eq!(unsafe { cons.pop_access(pop_fn_20) }.unwrap().unwrap(), (2, ()));


        let vs_11 = (123, 456);
        let push_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 1);
            left[0] = vs_11.0;
            right[0] = vs_11.1;
            Ok((2, ()))
        };

        assert_eq!(unsafe { prod.push_access(push_fn_11) }.unwrap().unwrap(), (2, ()));

        let pop_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
            assert_eq!(left.len(), 1);
            assert_eq!(right.len(), 1);
            assert_eq!(left[0], vs_11.0);
            assert_eq!(right[0], vs_11.1);
            Ok((2, ()))
        };

        assert_eq!(unsafe { cons.pop_access(pop_fn_11) }.unwrap().unwrap(), (2, ()));
    }

    #[test]
    fn push_pop_access_message() {
        let buf = RingBuffer::<u8>::new(7);
        let (mut prod, mut cons) = buf.split();

        let smsg = "The quick brown fox jumps over the lazy dog";
        
        let pjh = thread::spawn(move || {
            let mut bytes = smsg.as_bytes();
            while bytes.len() > 0 {
                let push_fn = |left: &mut [u8], right: &mut [u8]| -> Result<(usize, ()),()> {
                    let n = bytes.read(left).unwrap();
                    let m = bytes.read(right).unwrap();
                    Ok((n + m, ()))
                };
                match unsafe { prod.push_access(push_fn) } {
                    Ok(res) => match res {
                        Ok((_n, ())) => (),
                        Err(()) => unreachable!(),
                    },
                    Err(e) => match e {
                        PushAccessError::Full => thread::sleep(Duration::from_millis(1)),
                        PushAccessError::BadLen => unreachable!(),
                    }
                }
            }
            loop {
                match prod.push(0) {
                    Ok(()) => break,
                    Err((PushError::Full, _)) => thread::sleep(Duration::from_millis(1)),
                }
            }
        });

        let cjh = thread::spawn(move || {
            let mut bytes = Vec::<u8>::new();
            loop {
                let pop_fn = |left: &mut [u8], right: &mut [u8]| -> Result<(usize, ()),()> {
                    let n = bytes.write(left).unwrap();
                    let m = bytes.write(right).unwrap();
                    Ok((n + m, ()))
                };
                match unsafe { cons.pop_access(pop_fn) } {
                    Ok(res) => match res {
                        Ok((_n, ())) => (),
                        Err(()) => unreachable!(),
                    },
                    Err(e) => match e {
                        PopAccessError::Empty => {
                            if bytes.ends_with(&[0]) {
                                break;
                            } else {
                                thread::sleep(Duration::from_millis(1));
                            }
                        },
                        PopAccessError::BadLen => unreachable!(),
                    }
                }
            }

            assert_eq!(bytes.pop().unwrap(), 0);
            String::from_utf8(bytes).unwrap()
        });

        pjh.join().unwrap();
        let rmsg = cjh.join().unwrap();

        assert_eq!(smsg, rmsg);
    }

    #[test]
    fn push_pop_slice_message() {
        let buf = RingBuffer::<u8>::new(7);
        let (mut prod, mut cons) = buf.split();

        let smsg = "The quick brown fox jumps over the lazy dog";
        
        let pjh = thread::spawn(move || {
            let mut bytes = smsg.as_bytes();
            while bytes.len() > 0 {
                match prod.push_slice(bytes) {
                    Ok(n) => bytes = &bytes[n..bytes.len()],
                    Err(PushError::Full) => thread::sleep(Duration::from_millis(1)),
                }
            }
            loop {
                match prod.push(0) {
                    Ok(()) => break,
                    Err((PushError::Full, _)) => thread::sleep(Duration::from_millis(1)),
                }
            }
        });

        let cjh = thread::spawn(move || {
            let mut bytes = Vec::<u8>::new();
            let mut buffer = [0; 5];
            loop {
                match cons.pop_slice(&mut buffer) {
                    Ok(n) => bytes.extend_from_slice(&buffer[0..n]),
                    Err(PopError::Empty) => {
                        if bytes.ends_with(&[0]) {
                            break;
                        } else {
                            thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
            }

            assert_eq!(bytes.pop().unwrap(), 0);
            String::from_utf8(bytes).unwrap()
        });

        pjh.join().unwrap();
        let rmsg = cjh.join().unwrap();

        assert_eq!(smsg, rmsg);
    }
}

#[cfg(rustc_nightly)]
extern crate test;

#[cfg(rustc_nightly)]
#[cfg(test)]
mod benchmarks {
    use super::*;

    use test::Bencher;

    #[bench]
    fn push_pop(b: &mut Bencher) {
        let buf = RingBuffer::<u8>::new(0x100);
        let (mut prod, mut cons) = buf.split();
        b.iter(|| {
            while let Ok(()) = prod.push(0) {}
            while let Ok(0) = cons.pop() {}
        });
    }
}
