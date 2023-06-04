#![no_std]
#![allow(clippy::missing_safety_doc)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod halves;
pub mod rb;
pub mod traits;
mod transfer;

pub use alias::*;
pub use rb::AsyncRb;
pub use traits::{consumer, producer};
pub use transfer::async_transfer;

#[cfg(all(test, feature = "std"))]
mod tests;
