use crate::{consumer::GenericConsumer, producer::GenericProducer, utils::uninit_array};
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

#[cfg(feature = "alloc")]
use alloc::{sync::Arc, vec::Vec};

/// Abstract container for the ring buffer.
pub trait Container<T>: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> {}
impl<T, C> Container<T> for C where C: AsRef<[MaybeUninit<T>]> + AsMut<[MaybeUninit<T>]> {}

struct Storage<T, C: Container<T>> {
    len: usize,
    container: UnsafeCell<C>,
    phantom: PhantomData<T>,
}

unsafe impl<T, C: Container<T>> Sync for Storage<T, C> where T: Send {}

impl<T, C: Container<T>> Storage<T, C> {
    fn new(mut container: C) -> Self {
        Self {
            len: container.as_mut().len(),
            container: UnsafeCell::new(container),
            phantom: PhantomData,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn as_mut_slice(&self) -> &mut [MaybeUninit<T>] {
        (&mut *self.container.get()).as_mut()
    }
}

/// Reference to the ring buffer.
pub trait RingBufferRef<T, C: Container<T>>: Deref<Target = GenericRingBuffer<T, C>> {}
#[cfg(feature = "alloc")]
impl<T, C: Container<T>> RingBufferRef<T, C> for Arc<GenericRingBuffer<T, C>> {}
impl<'a, T, C: Container<T>> RingBufferRef<T, C> for &'a GenericRingBuffer<T, C> {}

/// Ring buffer itself.
///
/// The structure consists of abstract container (something that could be referenced as contiguous array) and two counters: `head` and `tail`.
/// When an element is extracted from the ring buffer it is taken from the head side. New elements are appended to the tail side.
///
/// The ring buffer does not take an extra space that means if its capacity is `N` then the container size is also `N` (not `N + 1`).
/// This is achieved by using modulus of `2 * Self::capacity()` (instead of `Self::capacity()`) for `head` and `tail` arithmetics.
/// It allows us to distinguish situations when the buffer is empty (`head == tail`) and when the buffer is full (`tail - head == Self::capacity()` modulo `2 * Self::capacity()`) without using an extra space in container.
pub struct GenericRingBuffer<T, C: Container<T>> {
    data: Storage<T, C>,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl<T, C: Container<T>> GenericRingBuffer<T, C> {
    pub(crate) fn head(&self) -> usize {
        self.head.load(Ordering::Acquire)
    }
    pub(crate) fn tail(&self) -> usize {
        self.tail.load(Ordering::Acquire)
    }

    /// Constructs ring buffer from container and counters.
    ///
    /// # Safety
    ///
    /// The items in container inside `head..tail` range must be initialized, items outside this range must be uninitialized.
    /// `head` and `tail` values must be valid (see structure documentaton).
    pub unsafe fn from_raw_parts(container: C, head: usize, tail: usize) -> Self {
        Self {
            data: Storage::new(container),
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    /// Splits ring buffer into producer and consumer.
    ///
    /// This method consumes the ring buffer and puts it on heap in `Arc`. If you don't want to use heap the see `split_static`.
    #[cfg(feature = "alloc")]
    #[allow(clippy::type_complexity)]
    pub fn split(
        self,
    ) -> (
        GenericProducer<T, C, Arc<Self>>,
        GenericConsumer<T, C, Arc<Self>>,
    ) {
        let arc = Arc::new(self);
        (GenericProducer::new(arc.clone()), GenericConsumer::new(arc))
    }

    /// Splits ring buffer into producer and consumer without using heap.
    ///
    /// In this case producer and consumer stores a reference to the ring buffer, so you need to store the buffer somewhere.
    pub fn split_static(&mut self) -> (GenericProducer<T, C, &Self>, GenericConsumer<T, C, &Self>) {
        (GenericProducer::new(self), GenericConsumer::new(self))
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
    pub unsafe fn advance_head(&self, count: usize) {
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
    pub unsafe fn advance_tail(&self, count: usize) {
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
        assert!(count <= left.len() + right.len());
        for elem in left.iter_mut().chain(right.iter_mut()).take(count) {
            unsafe { ptr::drop_in_place(elem.as_mut_ptr()) };
        }
        unsafe { self.advance_head(count) };
    }
}

impl<T, C: Container<T>> Drop for GenericRingBuffer<T, C> {
    fn drop(&mut self) {
        self.skip(self.occupied_len());
    }
}

/// Stack-allocated ring buffer with static capacity.
pub type StaticRingBuffer<T, const N: usize> = GenericRingBuffer<T, [MaybeUninit<T>; N]>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type RingBuffer<T> = GenericRingBuffer<T, Vec<MaybeUninit<T>>>;

#[cfg(feature = "alloc")]
impl<T> RingBuffer<T> {
    /// Creates a new instance of a ring buffer.
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize_with(capacity, MaybeUninit::uninit);
        unsafe { Self::from_raw_parts(data, 0, 0) }
    }
}

impl<T, const N: usize> Default for StaticRingBuffer<T, N> {
    fn default() -> Self {
        unsafe { Self::from_raw_parts(uninit_array(), 0, 0) }
    }
}

/// Moves at most `count` items from the `src` consumer to the `dst` producer.
/// Consumer and producer may be of different buffers as well as of the same one.
///
/// `count` is the number of items being moved, if `None` - as much as possible items will be moved.
///
/// Returns number of items been moved.
pub fn transfer<T, Cs, Cd, Rs, Rd>(
    src: &mut GenericConsumer<T, Cs, Rs>,
    dst: &mut GenericProducer<T, Cd, Rd>,
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
