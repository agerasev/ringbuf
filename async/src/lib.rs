//! Asynchronous SPSC FIFO ring buffers with runtime-independent synchronization.
//!
//! [`AsyncProducer`](traits::AsyncProducer) and [`AsyncConsumer`](traits::AsyncConsumer)
//! provide operations that wait for data, space, or closure of the opposite endpoint.
//! Their threshold waits return `Result<(), WaitError>`: [`WaitError::TooLarge`]
//! rejects thresholds above capacity, and [`WaitError::Closed`] reports peer closure.
//! A zero threshold succeeds immediately. Consumers can satisfy a wait from buffered
//! items after the producer closes.
//!
//! [`async_transfer`] moves items between buffers and returns the number delivered.
//! Cancelling it leaves undelivered items in the source buffer.
//!
//! Heap buffers provide `try_new`, which returns `ringbuf::CreateError` for invalid
//! capacity or allocation failure. Valid capacity is `1..=usize::MAX / 2`.
//!
#![no_std]
#![allow(clippy::missing_safety_doc)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
mod error;
pub mod rb;
pub mod traits;
mod transfer;
pub mod wrap;

pub use alias::*;
pub use error::WaitError;
pub use rb::AsyncRb;
pub use traits::{consumer, producer};
pub use transfer::async_transfer;

#[cfg(all(test, feature = "alloc"))]
mod tests;

#[cfg(all(test, feature = "bench"))]
mod bench;
