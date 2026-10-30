//! SPSC FIFO ring buffers with blocking operations and configurable timeouts.
//!
//! [`BlockingProd`] and [`BlockingCons`] wait for space or data using a semaphore.
//! Threshold waits return [`WaitError::TooLarge`] for requests above capacity,
//! [`WaitError::Closed`] for peer closure, and [`WaitError::TimedOut`] on timeout.
//! A zero threshold succeeds immediately. Consumers can satisfy a wait from buffered
//! items after the producer closes.
//!
//! Heap buffers provide `try_new`, which returns `ringbuf::CreateError` for invalid
//! capacity or allocation failure. Valid capacity is `1..=usize::MAX / 2`.
//!
#![no_std]
#![allow(clippy::missing_safety_doc)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod rb;
pub mod sync;
pub mod wrap;

#[cfg(all(test, feature = "std"))]
mod tests;

pub use ringbuf::traits;

#[cfg(feature = "alloc")]
pub use alias::*;
pub use rb::BlockingRb;
pub use wrap::{BlockingCons, BlockingProd, WaitError};
