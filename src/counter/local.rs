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
}

impl<'a, S: Counter> LocalTailCounter<'a, S> {
    /// Create `Self` with reference to some `global` counter.
    pub fn new(global: &'a S) -> Self {
        Self {
            local: LocalCounter::clone_from(global),
            global,
        }
    }
}
