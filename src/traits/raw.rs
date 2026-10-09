//! Unsafe implementation contracts underlying the safe endpoint APIs.
use super::{
    Observer,
    utils::{add_mod, modulus},
};
use core::mem::MaybeUninit;

/// Raw observer implementation.
///
/// # Safety
/// All observer methods and overrides used by safe operations must describe the
/// actual storage and endpoint bounds. Slices must refer to that stable storage,
/// respect wrapping, and have exactly the requested length. Shared data views
/// require exclusive endpoint ownership or an exclusive borrow of the RB.
pub unsafe trait RawObserver: Observer {
    /// Get slice between `start` and `end` indices.
    ///
    /// # Safety
    ///
    /// Slice must not overlap with any mutable slice existing at the same time.
    ///
    /// Non-`Sync` items must not be accessed from multiple threads at the same time.
    unsafe fn unsafe_slices(&self, start: usize, end: usize) -> (&[MaybeUninit<Self::Item>], &[MaybeUninit<Self::Item>]);

    /// Get mutable slice between `start` and `end` indices.
    ///
    /// # Safety
    ///
    /// There must not exist overlapping slices at the same time.
    #[allow(clippy::mut_from_ref)]
    unsafe fn unsafe_slices_mut(&self, start: usize, end: usize) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]);
}

/// Raw producer implementation.
///
/// # Safety
/// An exclusive borrow grants unique producer rights. Its bounds designate only
/// vacant slots. Publication must finish before returning or unwinding, including
/// when a notification panics. Safe Producer overrides must preserve these rules.
pub unsafe trait RawProducer: RawObserver {
    /// Set read index.
    ///
    /// # Safety
    ///
    /// Index must go only forward, never backward. It is recommended to use [`Self::advance_write_index`] instead.
    ///
    /// All slots with index less than `value` must be initialized until write index, all slots with index equal or greater - must be uninitialized.
    unsafe fn set_write_index(&self, value: usize);

    /// Moves `write` pointer by `count` places forward.
    ///
    /// # Safety
    ///
    /// First `count` items in free space must be initialized.
    ///
    /// Must not be called concurrently.
    unsafe fn advance_write_index(&self, count: usize) {
        unsafe { self.set_write_index(add_mod(self.write_index(), count, modulus(self))) };
    }
}

/// Raw consumer implementation.
///
/// # Safety
/// An exclusive borrow grants unique consumer rights. After prepare_read, its
/// bounds designate initialized, exclusively readable items. Release must finish
/// before returning or unwinding. Safe Consumer overrides must preserve these rules.
pub unsafe trait RawConsumer: RawObserver {
    /// Set read index.
    ///
    /// # Safety
    ///
    /// Index must go only forward, never backward. It is recommended to use [`Self::advance_read_index`] instead.
    ///
    /// All slots with index less than `value` must be uninitialized until write index, all slots with index equal or greater - must be initialized.
    unsafe fn set_read_index(&self, value: usize);

    /// Moves `read` pointer by `count` places forward.
    ///
    /// # Safety
    ///
    /// First `count` items in occupied memory must be moved out or dropped.
    ///
    /// Must not be called concurrently.
    unsafe fn advance_read_index(&self, count: usize) {
        unsafe { self.set_read_index(add_mod(self.read_index(), count, modulus(self))) };
    }

    /// Provides a direct mutable access to the ring buffer occupied memory.
    ///
    /// Same as [`Self::occupied_slices`].
    ///
    /// # Safety
    ///
    /// When some item is replaced with uninitialized value then it must not be read anymore.
    unsafe fn occupied_slices_mut(&mut self) -> (&mut [MaybeUninit<Self::Item>], &mut [MaybeUninit<Self::Item>]) {
        unsafe {
            self.prepare_read();
            self.unsafe_slices_mut(self.read_index(), self.write_index())
        }
    }

    /// Recover an abandoned reservation before ordinary data access.
    /// # Safety
    /// Caller has exclusive consumer access and no active deferred consumer.
    unsafe fn prepare_read(&mut self) {}
}

/// Raw ringbuffer implementation.
///
/// # Safety
/// Ownership flags must prevent duplicate endpoint acquisition, unless tracking
/// is disabled. Claim/release setters are bookkeeping only and must not panic or
/// run user code. Notifications occur separately after ownership is consistent.
pub unsafe trait RawRingBuffer: RawProducer + RawConsumer {
    /// Tell whether read end of the ring buffer is held by consumer or not.
    ///
    /// Returns old value.
    ///
    /// # Safety
    ///
    /// Must not be set to `false` while consumer exists.
    unsafe fn hold_read(&self, flag: bool) -> bool;
    /// Tell whether write end of the ring buffer is held by producer or not.
    ///
    /// Returns old value.
    ///
    /// # Safety
    ///
    /// Must not be set to `false` while producer exists.
    unsafe fn hold_write(&self, flag: bool) -> bool;

    /// # Safety
    /// Caller holds the consumer and transfers ownership of initialized slots.
    unsafe fn set_read_claimed(&self, value: usize);
    /// # Safety
    /// Caller holds the consumer and has finished accessing released slots.
    unsafe fn set_read_released(&self, value: usize);
    fn tracking_enabled(&self) -> bool {
        true
    }
    fn notify_read(&self) {}
    fn notify_write(&self) {}
}
