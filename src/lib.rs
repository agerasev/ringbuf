//! Lock-free single-producer single-consumer (SPSC) FIFO ring buffer with direct access to inner data.
//!
//! # Overview
//!
//! The initial thing you probably want to start with is to choose some implementation if the [`Rb`](`crate::ring_buffer::Rb`) trait representing ring buffer itself.
//!
//! Implementations of the [`Rb`](`crate::ring_buffer::Rb`) trait:
//!
//! + [`LocalRb`]. Only for single-threaded use.
//! + [`SharedRb`]. Can be shared between threads.
//!   + [`HeapRb`]. Contents are stored in dynamic memory. *Recommended for most use cases.*
//!   + [`StaticRb`]. Contents can be stored in statically-allocated memory.
//!
//! Ring buffer can be splitted into pair of [`Producer`] and [`Consumer`].
//!
//! [`Producer`] and [`Consumer`] are used to append/remove items to/from the ring buffer accordingly. For [`SharedRb`] they can be safely sent between threads.
//! Operations with [`Producer`] and [`Consumer`] are lock-free - they succeed or fail immediately without blocking or waiting.
//!
//! Elements can be effectively appended/removed one by one or many at once.
//! Also data could be loaded/stored directly into/from [`Read`](`std::io::Read`)/[`Write`](`std::io::Write`) instances.
//! And finally, there are `unsafe` methods allowing thread-safe direct access to the inner memory being appended/removed.
//!
//! When building with nightly toolchain it is possible to run benchmarks via `cargo bench --features bench`.
#![cfg_attr(
    feature = "alloc",
    doc = r##"
# Examples

## Simple example

```rust
# extern crate ringbuf;
use ringbuf::HeapRb;
# fn main() {
let rb = HeapRb::<i32>::new(2);
let (mut prod, mut cons) = rb.split();

prod.push(0).unwrap();
prod.push(1).unwrap();
assert_eq!(prod.push(2), Err(2));

assert_eq!(cons.pop(), Some(0));

prod.push(2).unwrap();

assert_eq!(cons.pop(), Some(1));
assert_eq!(cons.pop(), Some(2));
assert_eq!(cons.pop(), None);
# }
```
"##
)]
#![no_std]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod utils;

/// [`Consumer`] and additional types.
pub mod consumer;
/// [`Producer`] and additional types.
pub mod producer;
/// Ring buffer traits and implementations.
pub mod ring_buffer;
mod transfer;

pub use consumer::Consumer;
pub use producer::Producer;
#[cfg(feature = "alloc")]
pub use ring_buffer::HeapRb;
pub use ring_buffer::{LocalRb, SharedRb, StaticRb};
pub use transfer::transfer;

#[cfg(test)]
mod tests;

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
