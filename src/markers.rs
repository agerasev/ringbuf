//! Endpoint ownership and notification policies.
use core::cell::Cell;
#[cfg(not(feature = "portable-atomic"))]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "portable-atomic")]
use portable_atomic::{AtomicBool, Ordering};

/// Owns the endpoint presence flags.
///
/// # Safety
/// Implementations must atomically exclude duplicate acquisition of each role
/// (unless `TRACKED` is false). State transitions must finish before returning;
/// marker operations must not panic. `Sync` implementations synchronize handoff
/// between successive owners. Notifications may panic and run only after state
/// and item ownership have been updated.
pub unsafe trait Markers: Default {
    const TRACKED: bool = true;
    fn read_is_held(&self) -> bool;
    fn write_is_held(&self) -> bool;
    /// # Safety
    /// Only the current owner may release a role.
    unsafe fn hold_read(&self, held: bool) -> bool;
    /// # Safety
    /// Only the current owner may release a role.
    unsafe fn hold_write(&self, held: bool) -> bool;
    fn notify_read(&self) {}
    fn notify_write(&self) {}
}

/// Atomic endpoint presence flags.
#[derive(Default)]
pub struct AtomicMarkers {
    read: AtomicBool,
    write: AtomicBool,
}
/// Single-threaded endpoint presence flags.
#[derive(Default)]
pub struct LocalMarkers {
    read: Cell<bool>,
    write: Cell<bool>,
}

unsafe impl Markers for AtomicMarkers {
    #[inline]
    fn read_is_held(&self) -> bool {
        self.read.load(Ordering::Acquire)
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.write.load(Ordering::Acquire)
    }
    #[inline]
    unsafe fn hold_read(&self, held: bool) -> bool {
        self.read.swap(held, Ordering::AcqRel)
    }
    #[inline]
    unsafe fn hold_write(&self, held: bool) -> bool {
        self.write.swap(held, Ordering::AcqRel)
    }
}
unsafe impl Markers for LocalMarkers {
    #[inline]
    fn read_is_held(&self) -> bool {
        self.read.get()
    }
    #[inline]
    fn write_is_held(&self) -> bool {
        self.write.get()
    }
    #[inline]
    unsafe fn hold_read(&self, held: bool) -> bool {
        self.read.replace(held)
    }
    #[inline]
    unsafe fn hold_write(&self, held: bool) -> bool {
        self.write.replace(held)
    }
}

#[allow(unused_imports)]
use crate::traits::{RawConsumer, RawObserver, RawProducer, RawRingBuffer};
