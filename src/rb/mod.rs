#![doc = include_str!("../../docs/ownership.md")]

/// Single-threaded ring buffer implementation.
pub mod local;
mod macros;
/// Multi-threaded ring buffer implementation.
pub mod shared;
mod traits;
mod utils;

pub use local::LocalRb;
pub use shared::SharedRb;
pub use traits::*;

mod exclusive;
mod methods;
pub use exclusive::{Exclusive, IntoIter};
