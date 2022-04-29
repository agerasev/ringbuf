use alloc::{sync::Arc, vec::Vec};
use cache_padded::CachePadded;
use core::{
    cell::UnsafeCell,
    convert::{AsMut, AsRef},
    marker::PhantomData,
    mem::MaybeUninit,
    ops::Deref,
    ptr, slice,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{consumer::Consumer, producer::Producer};

pub trait Container<T>: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> {}
impl<T, C> Container<T> for C where C: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> {}

struct Storage<T, C: Container<T>> {
    len: usize,
    container: UnsafeCell<C>,
    phantom: PhantomData<T>,
}

unsafe impl<T, C: Container<T>> Sync for Storage<T, C> {}

impl<T, C: Container<T>> Storage<T, C> {
    fn new(mut container: C) -> Self {
        Self {
            len: container.as_mut().len(),
            container: UnsafeCell::new(container),
            phantom: PhantomData,
        }
    }

    fn into_inner(self) -> C {
        self.container.into_inner()
    }

    fn len(&self) -> usize {
        self.len
    }

    unsafe fn as_slice(&self) -> &[MaybeUninit<T>] {
        (&*self.container.get()).as_ref()
    }

    #[warn(clippy::mut_from_ref)]
    unsafe fn as_mut_slice(&self) -> &mut [MaybeUninit<T>] {
        (&mut *self.container.get()).as_mut()
    }
}

pub trait RingBufferRef<T, C: Container<T>>: Deref<Target = RingBuffer<T, C>> {}
impl<T, C: Container<T>> RingBufferRef<T, C> for Arc<RingBuffer<T, C>> {}
impl<'a, T, C: Container<T>> RingBufferRef<T, C> for &'a RingBuffer<T, C> {}

pub struct RingBuffer<T, C: Container<T>> {
    data: Storage<T, C>,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl<T, C: Container<T>> RingBuffer<T, C> {
    pub(crate) fn head(&self) -> usize {
        self.head.load(Ordering::Acquire)
    }
    pub(crate) fn tail(&self) -> usize {
        self.tail.load(Ordering::Acquire)
    }

    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            data: Storage::new(container),
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    /// Splits ring buffer into producer and consumer.
    pub fn split(self) -> (Producer<T, C, Arc<Self>>, Consumer<T, C, Arc<Self>>) {
        let arc = Arc::new(self);
        (Producer::new(arc.clone()), Consumer::new(arc))
    }

    pub fn split_ref(&mut self) -> (Producer<T, C, &Self>, Consumer<T, C, &Self>) {
        (Producer::new(self), Consumer::new(self))
    }

    /// The capacity of the ring buffer.
    ///
    /// This value does not change.
    pub fn capacity(&self) -> usize {
        self.data.len()
    }
    fn modulus(&self) -> usize {
        2 * self.capacity()
    }

    /// The number of elements stored in the buffer at the moment.
    pub fn occupied_len(&self) -> usize {
        (self.modulus() + self.tail() - self.head()) % self.modulus()
    }
    /// The number of vacant places in the buffer at the moment.
    pub fn vacant_len(&self) -> usize {
        (self.modulus() + self.head() - self.tail() - self.capacity()) % self.modulus()
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.occupied_len() == 0
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }

    /// Move ring buffer **head** pointer by `count` elements forward.
    ///
    /// # Safety
    ///
    /// First `count` elements in occupied area must be initialized before this call.
    ///
    /// *Panics if `count` is greater than number of elements in the ring buffer.*
    ///
    /// Allowed to call only from **consumer** side.
    pub unsafe fn shift_head(&self, count: usize) {
        assert!(count <= self.occupied_len());
        self.head
            .store((self.head() + count) % self.modulus(), Ordering::Release);
    }

    /// Move ring buffer **tail** pointer by `count` elements forward.
    ///
    /// # Safety
    ///
    /// First `count` elements in vacant area must be deinitialized (dropped) before this call.
    ///
    /// *Panics if `count` is greater than number of vacant places in the ring buffer.*
    ///
    /// Allowed to call only from **producer** side.
    pub unsafe fn shift_tail(&self, count: usize) {
        std::println!("count: {}, vacant: {}", count, self.vacant_len());
        assert!(count <= self.vacant_len());
        self.tail
            .store((self.tail() + count) % self.modulus(), Ordering::Release);
    }

    /// Returns a pair of slices which contain, in order, the occupied cells in the ring buffer.
    ///
    /// All elements in slices are guaranteed to be *initialized*.
    ///
    /// *The slices may not include elements pushed to the buffer by the concurring producer right after this call.*
    ///
    /// # Safety
    ///
    /// Allowed to call only from **consumer** side.
    pub unsafe fn occupied_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let head = self.head();
        let tail = self.tail();
        let len = self.data.len();

        let (head_div, head_mod) = (head / len, head % len);
        let (tail_div, tail_mod) = (tail / len, tail % len);

        let ranges = if head_div == tail_div {
            (head_mod..tail_mod, 0..0)
        } else {
            (head_mod..len, 0..tail_mod)
        };

        let ptr = self.data.as_mut_slice().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// Returns a pair of slices which contain, in order, the vacant cells in the ring buffer.
    ///
    /// All elements in slices are guaranteed to be *un-initialized*.
    ///
    /// *The slices may not include cells freed by the concurring consumer right after this call.*
    ///
    /// # Safety
    ///
    /// Allowed to call only from **producer** side.
    pub unsafe fn vacant_slices(&self) -> (&mut [MaybeUninit<T>], &mut [MaybeUninit<T>]) {
        let head = self.head();
        let tail = self.tail();
        let len = self.data.len();

        let (head_div, head_mod) = (head / len, head % len);
        let (tail_div, tail_mod) = (tail / len, tail % len);

        let ranges = if head_div == tail_div {
            (tail_mod..len, 0..head_mod)
        } else {
            (tail_mod..head_mod, 0..0)
        };

        let ptr = self.data.as_mut_slice().as_mut_ptr();
        (
            slice::from_raw_parts_mut(ptr.add(ranges.0.start), ranges.0.len()),
            slice::from_raw_parts_mut(ptr.add(ranges.1.start), ranges.1.len()),
        )
    }

    /// Removes exactly `count` elements from the head of ring buffer and drops them.
    ///
    /// *Panics if `count` is greater than number of elements stored in the buffer.*
    pub fn skip(&self, count: usize) {
        let (left, right) = unsafe { self.occupied_slices() };
        std::println!(
            "count: {}, left.len(): {}, right.len(): {}",
            count,
            left.len(),
            right.len()
        );
        assert!(count <= left.len() + right.len());
        for elem in left.iter_mut().chain(right.iter_mut()).take(count) {
            unsafe { ptr::drop_in_place(elem.as_mut_ptr()) };
        }
        unsafe { self.shift_head(count) };
    }
}

impl<T, C: Container<T>> Drop for RingBuffer<T, C> {
    fn drop(&mut self) {
        self.skip(self.occupied_len());
    }
}

pub type VecRingBuffer<T> = RingBuffer<T, Vec<MaybeUninit<T>>>;
pub type StaticRingBuffer<T, const N: usize> = RingBuffer<T, [MaybeUninit<T>; N]>;

impl<T> VecRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}

impl<T, const N: usize> Default for StaticRingBuffer<T, N> {
    fn default() -> Self {
        let uninit = MaybeUninit::<[T; N]>::uninit();
        let array = unsafe { (&uninit as *const _ as *const [MaybeUninit<T>; N]).read() };
        unsafe { Self::from_raw_parts(array, 0, 0) }
    }
}

/// Moves at most `count` items from the `src` consumer to the `dst` producer.
/// Consumer and producer may be of different buffers as well as of the same one.
///
/// `count` is the number of items being moved, if `None` - as much as possible items will be moved.
///
/// Returns number of items been moved.
pub fn transfer<T, Cs, Cd, Rs, Rd>(
    src: &mut Consumer<T, Cs, Rs>,
    dst: &mut Producer<T, Cd, Rd>,
    count: Option<usize>,
) -> usize
where
    Cs: Container<T>,
    Cd: Container<T>,
    Rs: RingBufferRef<T, Cs>,
    Rd: RingBufferRef<T, Cd>,
{
    let (src_left, src_right) = unsafe { src.as_uninit_slices() };
    let (dst_left, dst_right) = unsafe { dst.free_space_as_slices() };
    let src_iter = src_left.iter().chain(src_right.iter());
    let dst_iter = dst_left.iter_mut().chain(dst_right.iter_mut());

    let mut actual_count = 0;
    for (src_elem, dst_place) in src_iter.zip(dst_iter) {
        if let Some(count) = count {
            if actual_count >= count {
                break;
            }
        }
        unsafe { dst_place.write(src_elem.as_ptr().read()) };
        actual_count += 1;
    }
    unsafe { src.advance(actual_count) };
    unsafe { dst.advance(actual_count) };
    actual_count
}
