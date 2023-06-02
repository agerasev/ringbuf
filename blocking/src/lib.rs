#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
pub mod halves;
pub mod rb;
pub mod sync;
pub mod traits;

#[cfg(all(test, feature = "std"))]
mod tests;

pub use alias::*;
pub use halves::{BlockingCons, BlockingProd};
pub use rb::BlockingRb;
