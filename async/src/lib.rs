#![no_std]
#![allow(clippy::missing_safety_doc)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod rb;
pub mod traits;
mod transfer;
pub mod wrap;

pub use alias::*;
pub use rb::AsyncRb;
pub use traits::{consumer, producer};
pub use transfer::async_transfer;

#[cfg(all(test, feature = "alloc"))]
mod tests;

#[cfg(all(test, feature = "bench"))]
mod bench;
