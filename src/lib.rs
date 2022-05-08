//! Lock-free single-producer single-consumer (SPSC) FIFO ring buffer with direct access to inner data.
//!
//! # Overview
//!
//! `HeapRingBuffer` is the initial structure representing ring buffer itself.
//! Ring buffer can be splitted into pair of `HeapProducer` and `HeapConsumer`.
//!
//! `HeapProducer` and `HeapConsumer` are used to append/remove elements to/from the ring buffer accordingly. They can be safely sent between threads.
//! Operations with `HeapProducer` and `HeapConsumer` are lock-free - they succeed or fail immediately without blocking or waiting.
//!
//! Elements can be effectively appended/removed one by one or many at once.
//! Also data could be loaded/stored directly into/from [`Read`]/[`Write`] instances.
//! And finally, there are `unsafe` methods allowing thread-safe direct access in place to the inner memory being appended/removed.
//!
//! [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
//! [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
//!
//! When building with nightly toolchain it is possible to run benchmarks via `cargo bench --features benchmark`.
#![cfg_attr(
    feature = "alloc",
    doc = r##"
# Examples

## Simple example

```rust
# extern crate ringbuf;
use ringbuf::HeapRingBuffer;
# fn main() {
let rb = HeapRingBuffer::<i32>::new(2);
let (mut prod, mut cons) = rb.split();

prod.push(0).unwrap();
prod.push(1).unwrap();
assert_eq!(prod.push(2), Err(2));

assert_eq!(cons.pop().unwrap(), 0);

prod.push(2).unwrap();

assert_eq!(cons.pop().unwrap(), 1);
assert_eq!(cons.pop().unwrap(), 2);
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

mod consumer;
mod producer;
mod ring_buffer;
mod transfer;

pub use consumer::*;
pub use producer::*;
pub use ring_buffer::*;
pub use transfer::*;

#[cfg(test)]
mod tests;

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
