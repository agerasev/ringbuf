use alloc::{sync::Arc, vec::Vec};
use cache_padded::CachePadded;
use core::{
    cmp,
    mem::MaybeUninit,
    slice,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    consumer::{ArcConsumer, RefConsumer},
    producer::{ArcProducer, RefProducer},
    storage::{Container, Storage},
};

pub trait AbstractRingBuffer<T> {
    /// Returns capacity of the ring buffer.
    fn capacity(&self) -> usize;

    unsafe fn move_head(&self, count: usize);
    unsafe fn move_tail(&self, count: usize);

    /// All elements in slices are guaranteed to be *initialized*.
    unsafe fn occupied_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]);
    /// All elements in slices are guaranteed to be *uninitialized*.
    unsafe fn vacant_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]);

    /// The length of the data in the buffer.
    fn occupied_len(&self) -> usize;
    /// The remaining space in the buffer.
    fn vacant_len(&self) -> usize;

    /// Checks if the ring buffer is empty.
    fn is_empty(&self) -> bool {
        self.occupied_len() == 0
    }

    /// Checks if the ring buffer is full.
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }
}

pub struct RingBuffer<T, C: Container<MaybeUninit<T>>> {
    data: Storage<MaybeUninit<T>, C>,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}
impl<T, C: Container<MaybeUninit<T>>> RingBuffer<T, C> {
    fn head(&self) -> usize {
        self.head.load(Ordering::Acquire)
    }
    fn tail(&self) -> usize {
        self.head.load(Ordering::Acquire)
    }
    fn modulo(&self) -> usize {
        2 * self.capacity()
    }

    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            data: Storage::new(container),
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    /// Splits ring buffer into producer and consumer.
    pub fn split(self) -> (ArcProducer<T, Self>, ArcConsumer<T, Self>) {
        let arc = Arc::new(self);
        (ArcProducer::new(arc.clone()), ArcConsumer::new(arc))
    }

    pub fn split_ref(&mut self) -> (RefProducer<'_, T, Self>, RefConsumer<'_, T, Self>) {
        (RefProducer::new(self), RefConsumer::new(self))
    }
}

impl<T, C: Container<MaybeUninit<T>>> AbstractRingBuffer<T> for RingBuffer<T, C> {
    fn capacity(&self) -> usize {
        self.data.len()
    }

    unsafe fn move_head(&self, count: usize) {
        assert!(count <= self.occupied_len());
        self.head
            .store((self.head() + count) % self.modulo(), Ordering::Release);
    }
    unsafe fn move_tail(&self, count: usize) {
        assert!(count <= self.vacant_len());
        self.tail
            .store((self.tail() + count) % self.modulo(), Ordering::Release);
    }

    /// All elements in slices are guaranteed to be *initialized*.
    unsafe fn occupied_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let head = self.head();
        let tail = self.tail();
        let len = self.data.len();

        let ranges = match head.cmp(&tail) {
            cmp::Ordering::Less => ((head % len)..(tail % len), 0..0),
            cmp::Ordering::Greater => ((head % len)..len, 0..(tail % len)),
            cmp::Ordering::Equal => (0..0, 0..0),
        };

        let ptr = self.data.as_mut_slice().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// All elements in slices are guaranteed to be *uninitialized*.
    unsafe fn vacant_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let head = self.head();
        let tail = self.tail();
        let len = self.data.len();

        let ranges = match head.cmp(&tail) {
            cmp::Ordering::Less => ((tail % len)..len, 0..(head % len)),
            cmp::Ordering::Greater => ((tail % len)..(head % len), 0..0),
            cmp::Ordering::Equal => (0..0, 0..0),
        };

        let ptr = self.data.as_mut_slice().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// The number of elements stored in the buffer.
    fn occupied_len(&self) -> usize {
        (self.modulo() + self.tail() - self.head()) % self.modulo()
    }
    /// The number of vacant places in the buffer.
    fn vacant_len(&self) -> usize {
        (self.modulo() + self.head() - self.tail() - self.capacity()) % self.modulo()
    }
}

impl<T, C: Container<MaybeUninit<T>>> Drop for RingBuffer<T, C> {
    fn drop(&mut self) {
        let data = unsafe { self.data.as_mut_slice() };

        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        let len = data.len();

        let slices = if head <= tail {
            (head..tail, 0..0)
        } else {
            (head..len, 0..tail)
        };

        let drop = |elem_ref: &mut MaybeUninit<T>| unsafe {
            elem_ref.as_ptr().read();
        };
        for elem in data[slices.0].iter_mut() {
            drop(elem);
        }
        for elem in data[slices.1].iter_mut() {
            drop(elem);
        }
    }
}

impl<T> RingBuffer<T, Vec<MaybeUninit<T>>> {
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity + 1, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}

impl<T, const N: usize> Default for RingBuffer<T, [MaybeUninit<T>; N]> {
    fn default() -> Self {
        let uninit = MaybeUninit::<[T; N]>::uninit();
        let array = unsafe { (&uninit as *const _ as *const [MaybeUninit<T>; N]).read() };
        unsafe { Self::from_raw_parts(array, 0, 0) }
    }
}

/*
/// Moves at most `count` items from the `src` consumer to the `dst` producer.
/// Consumer and producer may be of different buffers as well as of the same one.
///
/// `count` is the number of items being moved, if `None` - as much as possible items will be moved.
///
/// Returns number of items been moved.
pub fn move_items<T>(src: &mut Consumer<T>, dst: &mut Producer<T>, count: Option<usize>) -> usize {
    unsafe {
        src.pop_access(|src_left, src_right| -> usize {
            dst.push_access(|dst_left, dst_right| -> usize {
                let n = count.unwrap_or_else(|| {
                    min(
                        src_left.len() + src_right.len(),
                        dst_left.len() + dst_right.len(),
                    )
                });
                let mut m = 0;
                let mut src = (SlicePtr::new(src_left), SlicePtr::new(src_right));
                let mut dst = (SlicePtr::new(dst_left), SlicePtr::new(dst_right));

                loop {
                    let k = min(n - m, min(src.0.len, dst.0.len));
                    if k == 0 {
                        break;
                    }
                    copy(src.0.ptr, dst.0.ptr, k);
                    if src.0.len == k {
                        src.0 = src.1;
                        src.1 = SlicePtr::null();
                    } else {
                        src.0.shift(k);
                    }
                    if dst.0.len == k {
                        dst.0 = dst.1;
                        dst.1 = SlicePtr::null();
                    } else {
                        dst.0.shift(k);
                    }
                    m += k
                }

                m
            })
        })
    }
}
*/
