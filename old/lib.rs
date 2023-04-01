//! Lock-free SPSC FIFO ring buffer with direct access to inner data.
//!
//! # Features
//!
//! + Lock-free operations - they succeed or fail immediately without blocking or waiting.
//! + Arbitrary item type (not only [`Copy`]).
//! + Items can be inserted and removed one by one or many at once.
//! + Thread-safe direct access to the internal ring buffer memory.
//! + [`Read`](`std::io::Read`) and [`Write`](`std::io::Write`) implementation.
//! + Can be used without `std` and even without `alloc` (using only statically-allocated memory).
//! + [Experimental `async`/`.await` support](https://github.com/agerasev/async-ringbuf).
//!
//! # Usage
//!
//! At first you need to create the ring buffer itself. [`HeapRb`] is recommended but you may [choose another one](#types).
//!
//! After the ring buffer is created it may be splitted into pair of [`Producer`] and [`Consumer`].
//! [`Producer`] is used to insert items to the ring buffer, [`Consumer`] - to remove items from it.
//! For [`SharedRb`] and its derivatives they can be used in different threads.
//!
//! Also you can use the ring buffer without splitting at all via methods provided by [`Rb`] trait.
//!
//! # Types
//!
//! There are several types of ring buffers provided:
//!
//! + [`LocalRb`]. Only for single-threaded use.
//! + [`SharedRb`]. Can be shared between threads. Its derivatives:
//!   + [`HeapRb`]. Contents are stored in dynamic memory. *Recommended for use in most cases.*
//!   + [`StaticRb`]. Contents can be stored in statically-allocated memory.
//!
//! # Performance
//!
//! [`SharedRb`] needs to synchronize CPU cache between CPU cores. This synchronization has some overhead.
//! To avoid multiple unnecessary synchronizations you may use postponed mode of operation (see description for [`Producer#mode`] and [`Consumer#mode`])
//! or methods that operate many items at once ([`Producer::push_slice`]/[`Producer::push_iter`], [`Consumer::pop_slice`], etc.).
//!
//! For single-threaded usage [`LocalRb`] is recommended because it is faster than [`SharedRb`] due to absence of CPU cache synchronization.
//!
//! ## Benchmarks
//!
//! You may see typical performance of different methods in benchmarks:
//!
//! ```bash
//! cargo +nightly bench --features bench
//! ```
//!
//! Nightly toolchain is required.
//!
//! # Examples
//!
#![cfg_attr(
    feature = "alloc",
    doc = r##"
## Simple

```rust
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
#![doc = r##"
## No heap

```rust
use ringbuf::StaticRb;

# fn main() {
const RB_SIZE: usize = 1;
let mut rb = StaticRb::<i32, RB_SIZE>::default();
let (mut prod, mut cons) = rb.split_ref();

assert_eq!(prod.push(123), Ok(()));
assert_eq!(prod.push(321), Err(321));

assert_eq!(cons.pop(), Some(123));
assert_eq!(cons.pop(), None);
# }
```
"##]
#![cfg_attr(
    feature = "std",
    doc = r##"
## Overwrite

Ring buffer can be used in overwriting mode when insertion overwrites the latest element if the buffer is full.

```rust
use ringbuf::{HeapRb, Rb};

# fn main() {
let mut rb = HeapRb::<i32>::new(2);

assert_eq!(rb.push_overwrite(0), None);
assert_eq!(rb.push_overwrite(1), None);
assert_eq!(rb.push_overwrite(2), Some(0));

assert_eq!(rb.pop(), Some(1));
assert_eq!(rb.pop(), Some(2));
assert_eq!(rb.pop(), None);
# }
```

Note that [`push_overwrite`](`Rb::push_overwrite`) requires exclusive access to the ring buffer
so to perform it concurrently you need to guard the ring buffer with [`Mutex`](`std::sync::Mutex`) or some other lock.
"##
)]
//! ## `async`/`.await`
//!
//! There is an experimental crate [`async-ringbuf`](https://github.com/agerasev/async-ringbuf)
//! which is built on top of `ringbuf` and implements asynchronous ring buffer operations.
//!
#![no_std]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod alias;
mod utils;

/// [`Consumer`] and additional types.
pub mod consumer;
/// [`Producer`] and additional types.
pub mod producer;
/// Ring buffer traits and implementations.
pub mod ring_buffer;
mod transfer;

#[cfg(feature = "alloc")]
pub use alias::{HeapConsumer, HeapProducer, HeapRb};
pub use alias::{StaticConsumer, StaticProducer, StaticRb};
pub use consumer::Consumer;
pub use producer::Producer;
pub use ring_buffer::{LocalRb, Rb, SharedRb};
pub use transfer::transfer;

#[cfg(test)]
mod tests;

#[cfg(feature = "bench")]
extern crate test;
#[cfg(feature = "bench")]
mod benchmarks;
