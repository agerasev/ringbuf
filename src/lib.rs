//! Lock-free SPSC FIFO ring buffer with direct access to inner data.
//!
//! For implementation details see [`RingBuffer`](`traits::RingBuffer`) description.
#![no_std]
#![allow(clippy::type_complexity)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

/// Shortcuts for frequently used types.
mod alias;
/// Ring buffer implementations.
pub mod rb;
/// Storage types.
pub mod storage;
/// Ring buffer traits.
pub mod traits;
/// Items transfer between ring buffers.
mod transfer;
/// Internal utilities.
mod utils;
/// Producer and consumer implementations.
pub mod wrap;

#[cfg(test)]
mod tests;

pub use alias::*;
pub use rb::{LocalRb, SharedRb};
pub use traits::{consumer, producer};
pub use transfer::transfer;
pub use wrap::{CachingCons, CachingProd, Cons, Obs, Prod};

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
