#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
mod alias;
pub mod halves;
pub mod rb;
pub mod traits;
mod transfer;

#[cfg(feature = "alloc")]
pub use alias::*;
pub use rb::AsyncRb;
pub use traits::{consumer, producer};
pub use transfer::async_transfer;

#[cfg(all(test, feature = "std"))]
mod tests;
