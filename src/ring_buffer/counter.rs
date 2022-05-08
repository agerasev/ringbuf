use cache_padded::CachePadded;
use core::{
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
    sync::atomic::{AtomicUsize, Ordering},
};

pub trait Counter {
    fn len(&self) -> NonZeroUsize;
    fn head(&self) -> usize;
    fn tail(&self) -> usize;

    #[inline]
    fn modulus(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(2 * self.len().get()) }
    }

    /// The number of elements stored in the buffer at the moment.
    fn occupied_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.tail() - self.head()) % modulus
    }

    /// The number of vacant places in the buffer at the moment.
    fn vacant_len(&self) -> usize {
        let modulus = self.modulus();
        (modulus.get() + self.head() - self.tail() - self.len().get()) % modulus
    }

    /// Checks if the occupied range is empty.
    fn is_empty(&self) -> bool {
        self.head() == self.tail()
    }

    /// Checks if the vacant range is empty.
    fn is_full(&self) -> bool {
        self.vacant_len() == 0
    }
}

pub struct LocalCounter {
    pub len: NonZeroUsize,
    pub head: usize,
    pub tail: usize,
}

impl Counter for LocalCounter {
    fn len(&self) -> NonZeroUsize {
        self.len
    }
    fn head(&self) -> usize {
        self.head
    }
    fn tail(&self) -> usize {
        self.tail
    }
}

pub struct HeadCounter<'a> {
    global: &'a GlobalCounter,
    local: LocalCounter,
}
impl<'a> Drop for HeadCounter<'a> {
    fn drop(&mut self) {
        self.global.store_head(self.local.head);
    }
}
impl<'a> Deref for HeadCounter<'a> {
    type Target = LocalCounter;
    fn deref(&self) -> &LocalCounter {
        &self.local
    }
}
impl<'a> DerefMut for HeadCounter<'a> {
    fn deref_mut(&mut self) -> &mut LocalCounter {
        &mut self.local
    }
}

pub struct TailCounter<'a> {
    global: &'a GlobalCounter,
    local: LocalCounter,
}
impl<'a> Drop for TailCounter<'a> {
    fn drop(&mut self) {
        self.global.store_tail(self.local.tail);
    }
}
impl<'a> Deref for TailCounter<'a> {
    type Target = LocalCounter;
    fn deref(&self) -> &LocalCounter {
        &self.local
    }
}
impl<'a> DerefMut for TailCounter<'a> {
    fn deref_mut(&mut self) -> &mut LocalCounter {
        &mut self.local
    }
}

impl<'a> HeadCounter<'a> {
    /// Returns a pair of slices which contain, in order, the occupied cells in the ring buffer.
    ///
    /// All elements in slices are guaranteed to be *initialized*.
    ///
    /// *The slices may not include elements pushed to the buffer by the concurring producer right after this call.*
    pub fn occupied_ranges(&self) -> (Range<usize>, Range<usize>) {
        let head = self.head();
        let tail = self.tail();
        let len = self.len();

        let (head_div, head_mod) = (head / len, head % len);
        let (tail_div, tail_mod) = (tail / len, tail % len);

        if head_div == tail_div {
            (head_mod..tail_mod, 0..0)
        } else {
            (head_mod..len.get(), 0..tail_mod)
        }
    }
    /// Move ring buffer **head** pointer by `count` elements forward.
    ///
    /// # Safety
    ///
    /// First `count` elements in occupied area must be initialized before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of elements in the ring buffer.*
    pub unsafe fn advance_head(&mut self, count: usize) {
        debug_assert!(count <= self.occupied_len());
        self.head = (self.head() + count) % self.modulus();
    }
}

impl<'a> TailCounter<'a> {
    /// Returns a pair of slices which contain, in order, the vacant cells in the ring buffer.
    ///
    /// All elements in slices are guaranteed to be *un-initialized*.
    ///
    /// *The slices may not include cells freed by the concurring consumer right after this call.*
    pub fn vacant_ranges(&self) -> (Range<usize>, Range<usize>) {
        let head = self.head();
        let tail = self.tail();
        let len = self.len();

        let (head_div, head_mod) = (head / len, head % len);
        let (tail_div, tail_mod) = (tail / len, tail % len);

        if head_div == tail_div {
            (tail_mod..len.get(), 0..head_mod)
        } else {
            (tail_mod..head_mod, 0..0)
        }
    }

    /// Move ring buffer **tail** pointer by `count` elements forward.
    ///
    /// # Safety
    ///
    /// First `count` elements in vacant area must be deinitialized (dropped) before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of vacant places in the ring buffer.*
    pub unsafe fn advance_tail(&mut self, count: usize) {
        debug_assert!(count <= self.vacant_len());
        self.tail = (self.tail() + count) % self.modulus();
    }
}

pub struct GlobalCounter {
    len: NonZeroUsize,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

impl Counter for GlobalCounter {
    fn len(&self) -> NonZeroUsize {
        self.len
    }
    fn head(&self) -> usize {
        self.head.load(Ordering::Acquire)
    }
    fn tail(&self) -> usize {
        self.tail.load(Ordering::Acquire)
    }
}

impl GlobalCounter {
    pub fn new(len: NonZeroUsize, head: usize, tail: usize) -> Self {
        Self {
            len,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    fn store_head(&self, value: usize) {
        self.head.store(value, Ordering::Release)
    }

    fn store_tail(&self, value: usize) {
        self.tail.store(value, Ordering::Release)
    }

    pub unsafe fn acquire_head(&self) -> HeadCounter<'_> {
        HeadCounter {
            local: LocalCounter {
                len: self.len(),
                head: self.head(),
                tail: self.tail(),
            },
            global: self,
        }
    }

    pub unsafe fn acquire_tail(&self) -> TailCounter<'_> {
        TailCounter {
            local: LocalCounter {
                len: self.len(),
                head: self.head(),
                tail: self.tail(),
            },
            global: self,
        }
    }
}
