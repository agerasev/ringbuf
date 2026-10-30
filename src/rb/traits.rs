use crate::traits::RingBuffer;
#[cfg(feature = "alloc")]
use {crate::alias::Arc, alloc::rc::Rc};

/// A buffer or handle providing shared access to a backing ring buffer.
///
/// # Safety
///
/// Every access through `rb` must refer to the same live backing buffer. The
/// buffer must outlive every reference returned from the handle. `rb` must not
/// unwind. A handle does not itself grant producer or consumer rights.
///
/// If the handle implements `Clone`, every clone must refer to that same buffer;
/// cloning must not copy the buffer into independent storage.
pub unsafe trait RbRef {
    /// Underlying ring buffer.
    type Rb: RingBuffer + ?Sized;
    /// Get a shared reference to the backing buffer.
    fn rb(&self) -> &Self::Rb;
}

/// A buffer or handle owning exclusive access to a backing ring buffer.
///
/// # Safety
///
/// An adapter owning this handle must retain exclusive access to both endpoint
/// roles while it provides shared data views. Any other access to the buffer
/// must be restricted to borrows derived from this handle, so those borrows
/// prevent moving it or borrowing it mutably. It is not enough to check for
/// uniqueness only when `rb_mut` is called.
///
/// `rb_mut` and [`RbRef::rb`] must access the same buffer. Exclusive access must
/// be valid even if previously borrowed endpoints were forgotten and left held
/// flags set. Mutable access may replace the buffer; subsequent accesses must
/// describe the replacement.
///
/// Shared handles such as `Arc` and `Rc` do not satisfy this contract.
pub unsafe trait RbMut: RbRef {
    /// Get an exclusive reference to the backing buffer.
    fn rb_mut(&mut self) -> &mut Self::Rb;
}

// SAFETY: the reference always accesses its referent, including when cloned.
unsafe impl<B: RingBuffer + ?Sized> RbRef for &B {
    type Rb = B;
    fn rb(&self) -> &B {
        self
    }
}

// SAFETY: both accessors reborrow the same buffer directly, without relying on
// user-defined AsRef or AsMut implementations.
unsafe impl<B: RingBuffer + ?Sized> RbRef for &mut B {
    type Rb = B;
    fn rb(&self) -> &B {
        self
    }
}
// SAFETY: the mutable reference owns both roles under RingBuffer's contract;
// all derived borrows prevent moving or mutably reborrowing the handle.
unsafe impl<B: RingBuffer + ?Sized> RbMut for &mut B {
    fn rb_mut(&mut self) -> &mut B {
        self
    }
}

#[cfg(feature = "alloc")]
// SAFETY: Rc and its clones refer to the same live buffer.
unsafe impl<B: RingBuffer + ?Sized> RbRef for Rc<B> {
    type Rb = B;
    fn rb(&self) -> &B {
        self
    }
}
#[cfg(feature = "alloc")]
// SAFETY: Arc and its clones refer to the same live buffer.
unsafe impl<B: RingBuffer + ?Sized> RbRef for Arc<B> {
    type Rb = B;
    fn rb(&self) -> &B {
        self
    }
}
