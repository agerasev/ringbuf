use super::{Counter, DefaultCounter};
use core::{
    cell::Cell,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
};

/// Non-thread-safe counter.
///
/// May be exclusively owned only by single thread.
pub struct LocalCounter {
    len: NonZeroUsize,
    head: Cell<usize>,
    tail: Cell<usize>,
}

impl LocalCounter {
    /// Clone state of another counter.
    pub fn clone_from<S: Counter>(other: &S) -> Self {
        Self::new(other.len(), other.head(), other.tail())
    }
}

impl LocalCounter {
    /// Construct new counter from capacity and head and tail positions.
    ///
    /// Panics if `tail - head` modulo `2 * len` is greater than `len`.
    fn new(len: NonZeroUsize, head: usize, tail: usize) -> Self {
        let self_ = Self {
            len,
            head: Cell::new(head),
            tail: Cell::new(tail),
        };
        assert!(self_.occupied_len() <= len.get());
        self_
    }
}

impl Counter for LocalCounter {
    fn len(&self) -> NonZeroUsize {
        self.len
    }
    fn head(&self) -> usize {
        self.head.get()
    }
    fn tail(&self) -> usize {
        self.tail.get()
    }

    unsafe fn set_head(&self, value: usize) {
        self.head.set(value);
    }
    unsafe fn set_tail(&self, value: usize) {
        self.tail.set(value);
    }
}

impl DefaultCounter for LocalCounter {
    fn with_capacity(len: NonZeroUsize) -> Self {
        Self::new(len, 0, 0)
    }
}

/// Local copy of some another counter that writes its **head** position to its origin when being dropped.
pub struct LocalHeadCounter<'a, S: Counter> {
    global: &'a S,
    local: LocalCounter,
}
impl<'a, S: Counter> Drop for LocalHeadCounter<'a, S> {
    fn drop(&mut self) {
        unsafe { self.global.set_head(self.local.head.get()) };
    }
}
impl<'a, S: Counter> Deref for LocalHeadCounter<'a, S> {
    type Target = LocalCounter;
    fn deref(&self) -> &LocalCounter {
        &self.local
    }
}
impl<'a, S: Counter> DerefMut for LocalHeadCounter<'a, S> {
    fn deref_mut(&mut self) -> &mut LocalCounter {
        &mut self.local
    }
}

/// Local copy of some another counter that writes its **tail** position to its origin when being dropped.
pub struct LocalTailCounter<'a, S: Counter> {
    global: &'a S,
    local: LocalCounter,
}
impl<'a, S: Counter> Drop for LocalTailCounter<'a, S> {
    fn drop(&mut self) {
        unsafe { self.global.set_tail(self.local.tail.get()) };
    }
}
impl<'a, S: Counter> Deref for LocalTailCounter<'a, S> {
    type Target = LocalCounter;
    fn deref(&self) -> &LocalCounter {
        &self.local
    }
}
impl<'a, S: Counter> DerefMut for LocalTailCounter<'a, S> {
    fn deref_mut(&mut self) -> &mut LocalCounter {
        &mut self.local
    }
}

impl<'a, S: Counter> LocalHeadCounter<'a, S> {
    /// Create `Self` with reference to some `global` counter.
    pub fn new(global: &'a S) -> Self {
        Self {
            local: LocalCounter::clone_from(global),
            global,
        }
    }

    /// Returns a pair of slices which contain, in order, the occupied cells in the ring buffer.
    ///
    /// All items in slices are guaranteed to be **initialized**.
    ///
    /// *The slices may not include items pushed to the buffer by the concurring producer right after this call.*
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
    /// Move **head** position by `count` items forward.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied area must be **initialized** before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of items in the ring buffer.*
    pub unsafe fn advance_head(&mut self, count: usize) {
        debug_assert!(count <= self.occupied_len());
        self.head.set((self.head() + count) % self.modulus());
    }
}

impl<'a, S: Counter> LocalTailCounter<'a, S> {
    /// Create `Self` with reference to some `global` counter.
    pub fn new(global: &'a S) -> Self {
        Self {
            local: LocalCounter::clone_from(global),
            global,
        }
    }

    /// Returns a pair of slices which contain, in order, the vacant cells in the ring buffer.
    ///
    /// All items in slices are guaranteed to be *un-initialized*.
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

    /// Move **tail** position by `count` items forward.
    ///
    /// # Safety
    ///
    /// First `count` items in vacant area must be **de-initialized** (dropped) before this call.
    ///
    /// *In debug mode panics if `count` is greater than number of vacant places in the ring buffer.*
    pub unsafe fn advance_tail(&mut self, count: usize) {
        debug_assert!(count <= self.vacant_len());
        self.tail.set((self.tail() + count) % self.modulus());
    }
}
