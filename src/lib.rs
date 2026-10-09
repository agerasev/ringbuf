//! Lock-free SPSC FIFO ring buffer with direct access to inner data.
//!
//! # Usage
//!
//! At first you need to create the ring buffer itself. [`HeapRb`] is recommended but you may [choose another one](#types).
//!
//! After the ring buffer is created it may be splitted into pair of [`Producer`](`traits::Producer`) and [`Consumer`](`traits::Consumer`).
//! Producer is used to insert items to the ring buffer, consumer - to remove items from it.
//!
//! # Types
//!
//! There are several types of ring buffers provided:
//!
//! + [`LocalRb`]. Only for single-threaded use.
//! + [`SharedRb`]. Can be shared between threads. Its frequently used instances:
//!   + [`HeapRb`]. Contents are stored in dynamic memory. *Recommended for use in most cases.*
//!   + [`StaticRb`]. Contents can be stored in statically-allocated memory.
//!
//! You may also provide your own generic parameters.
//!
//! # Performance
//!
//! [`SharedRb`] needs to synchronize CPU cache between CPU cores. This synchronization has some overhead.
//! To avoid multiple unnecessary synchronizations you may use methods that operate many items at once
//! ([`push_slice`](`traits::Producer::push_slice`)/[`push_iter`](`traits::Producer::push_iter`), [`pop_slice`](`traits::Consumer::pop_slice`), etc.).
//! Cached endpoints also avoid repeatedly fetching the opposite endpoint's index when progress is possible.
//! Direct and cached endpoints publish updates immediately, including each step of [`pop_iter`](`traits::Consumer::pop_iter`).
//! [`endpoint::Deferred`] endpoints batch publication with explicit commit/fetch/sync.
//! Their destructor commits; forgetting one may leak reserved values safely.
//!
//! [`skip`](`traits::Consumer::skip`) and [`clear`](`traits::Consumer::clear`) take constant time for items without destructors.
//! Items that need destruction are dropped individually, with each slot kept occupied until its destructor finishes.
//!
//! [`LocalRb`] selects compact `Cell` indices; [`SharedRb`] selects atomic indices.
//! Both alias [`Rb`]; measure the relevant workload before selecting a policy.
//!
//! # Examples
//!
#![cfg_attr(
    feature = "alloc",
    doc = r##"
## Simple

```rust
use ringbuf::{traits::*, HeapRb};

# fn main() {
let rb = HeapRb::<i32>::new(2);
let (mut prod, mut cons) = rb.split();

prod.try_push(0).unwrap();
prod.try_push(1).unwrap();
assert_eq!(prod.try_push(2), Err(2));

assert_eq!(cons.try_pop(), Some(0));

prod.try_push(2).unwrap();

assert_eq!(cons.try_pop(), Some(1));
assert_eq!(cons.try_pop(), Some(2));
assert_eq!(cons.try_pop(), None);
# }
```
"##
)]
#![doc = r##"
## No heap

```rust
use ringbuf::{traits::*, StaticRb};

# fn main() {
const RB_SIZE: usize = 1;
let mut rb = StaticRb::<i32, RB_SIZE>::default();
let (mut prod, mut cons) = rb.split_ref();

assert_eq!(prod.try_push(123), Ok(()));
assert_eq!(prod.try_push(321), Err(321));

assert_eq!(cons.try_pop(), Some(123));
assert_eq!(cons.try_pop(), None);
# }
```
"##]
#![cfg_attr(
    feature = "std",
    doc = r##"
## Overwrite

Ring buffer can be used in overwriting mode when insertion overwrites the oldest element if the buffer is full.

```rust
use ringbuf::{traits::*, HeapRb};

# fn main() {
let mut rb = HeapRb::<i32>::new(2);

assert_eq!(rb.push_overwrite(0), None);
assert_eq!(rb.push_overwrite(1), None);
assert_eq!(rb.push_overwrite(2), Some(0));

assert_eq!(rb.try_pop(), Some(1));
assert_eq!(rb.try_pop(), Some(2));
assert_eq!(rb.try_pop(), None);
# }
```

Note that [`push_overwrite`](`traits::RingBuffer::push_overwrite`) requires exclusive access to the ring buffer
so to perform it concurrently you need to guard the ring buffer with mutex or some other lock.
"##
)]
//!
//! # Implementation details
//!
//! Each ring buffer here consists of the following parts:
//!
//! + Storage
//! + Indices
//! + Markers
//!
//! ## Storage
//!
//! [`Storage`](`storage::Storage`) is a place where ring buffer items are actually stored.
//! It must span a single contiguous memory area (e.g. we can obtain a slice or subslice of it).
//! Ring buffer can own its storage or it can hold only a mutable reference to it.
//! Storage length is refered as `capacity`.
//! Capacity must be in `1..=usize::MAX / 2`, including for zero-sized items.
//!
//! ## Indices
//!
//! The backing RB stores three indices, modulo twice its capacity:
//!
//! - `read_released`: oldest slot still retained by the consumer.
//! - `read_claimed`: first initialized item still owned by the RB.
//! - `write_published`: end of the published items.
//!
//! In logical unwrapped coordinates the invariant is
//! `read_released <= read_claimed <= write_published <= read_released + capacity`.
//! Deferred readers claim initialized items before moving any out. Forgetting a
//! reader leaves that claim excluded from the RB destructor; the next ordinary
//! consumer access abandons it without inspecting potentially moved values.
//! Deferred writers own their unpublished values; forgetting them can leak those values.
//! [`traits::Observer::retained_len`] includes claims, while
//! [`traits::Observer::queued_len`] counts only items still owned by the RB.
//! Concurrent observer statistics are advisory rather than a coherent snapshot.
//!
//! ## Markers
//!
//! Ring buffer can have at most one producer and at most one consumer at the same time.
//! Tracked marker policies permit checked acquisition and optional presence queries.
//! [`markers::NoMarkers`] omits flags and requires owned or exclusive-borrow splitting.
//! Async and blocking marker policies also notify on publication, release, and drop.
//! Basic push/pop operations ignore whether the opposite endpoint is present.
//!
#![no_std]
#![allow(clippy::type_complexity)]
#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

/// Shortcuts for frequently used types.
mod alias;
/// Producer and consumer implementations.
pub mod endpoint;
pub mod indices;
pub mod markers;
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
pub use endpoint::{CachedCons, CachedProd, Cons, Obs, Prod};
pub use rb::{LocalRb, Rb, SharedRb};
pub use traits::{consumer, producer};
pub use transfer::transfer;

#[cfg(feature = "bench")]
extern crate test;
#[cfg(all(feature = "bench", test))]
mod benchmarks;

/// Compatibility module; new code should use `endpoint`.
pub use endpoint as wrap;

pub use endpoint::{CachingCons, CachingProd};

pub mod error;
pub use endpoint::{DeferredCons, DeferredProd, DirectCons, DirectProd};
#[cfg(feature = "alloc")]
pub use error::CreateError;
pub use error::{CapacityError, ExactError};
pub use rb::RbHandle;
