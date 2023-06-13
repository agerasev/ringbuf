#![no_std]
#![allow(clippy::type_complexity)]
#![allow(clippy::missing_safety_doc)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

/// Shortcuts for frequently used types.
mod alias;
/// Producer and consumer implementations.
pub mod halves;
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

#[cfg(test)]
mod tests;

pub use alias::*;
pub use halves::{CachingCons, CachingProd, Cons, Obs, Prod};
pub use rb::{LocalRb, SharedRb};
pub use traits::{consumer, producer};
pub use transfer::transfer;

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
