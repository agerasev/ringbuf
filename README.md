> The `v6` branch contains the experimental 0.6 API. See
> [migration notes](docs/migration-0.6.md), [design and milestones](docs/design-0.6.md),
> and the [deferred endpoint example](examples/deferred.rs).

# ringbuf

[![Crates.io][crates_badge]][crates]
[![Docs.rs][docs_badge]][docs]
[![Github Actions][github_badge]][github]
[![Gitlab CI][gitlab_badge]][gitlab]
[![License][license_badge]][license]

[crates_badge]: https://img.shields.io/crates/v/ringbuf.svg
[docs_badge]: https://docs.rs/ringbuf/badge.svg
[github_badge]: https://github.com/agerasev/ringbuf/actions/workflows/test.yml/badge.svg
[gitlab_badge]: https://gitlab.com/agerasev/ringbuf/badges/master/pipeline.svg
[license_badge]: https://img.shields.io/crates/l/ringbuf.svg

[crates]: https://crates.io/crates/ringbuf
[docs]: https://docs.rs/ringbuf
[github]: https://github.com/agerasev/ringbuf/actions/workflows/test.yml
[gitlab]: https://gitlab.com/agerasev/ringbuf/-/pipelines?scope=branches&ref=master
[license]: #license

Lock-free SPSC FIFO ring buffer with direct access to inner data.

## Features

+ Lock-free operations - they succeed or fail immediately without blocking or waiting.
+ Arbitrary item type (not only `Copy`).
+ Items can be inserted and removed one by one or many at once.
+ Thread-safe direct access to the internal ring buffer memory.
+ `Read` and `Write` implementation.
+ Overwriting insertion support.
+ Different types of buffers and underlying storages.
+ Can be used without `std` and even without `alloc` (using only statically-allocated memory).
+ Async and blocking versions (see [this section](#derived-crates)).
+ Can optionally use the [`portable-atomic`](https://crates.io/crates/portable-atomic) crate to allow usage on smaller systems without CAS operations.

# Usage

At first you need to create the ring buffer itself. `HeapRb` is recommended but you may [choose another one](#types).

After the ring buffer is created it may be splitted into pair of `Producer` and `Consumer`.
Producer is used to insert items to the ring buffer, consumer - to remove items from it.

Capacity must be in `1..=usize::MAX / 2`, including for zero-sized items.
Constructors panic if the capacity is zero or exceeds this limit.

# Types

There are several types of ring buffers provided:

+ `LocalRb`. Only for single-threaded use.
+ `SharedRb`. Can be shared between threads. Its frequently used instances:
  + `HeapRb`. Contents are stored in dynamic memory. *Recommended for use in most cases.*
  + `StaticRb`. Contents can be stored in statically-allocated memory.

You may also provide your own generic parameters.

# Performance

`SharedRb` needs to synchronize CPU cache between CPU cores. This synchronization has some overhead.
To avoid multiple unnecessary synchronizations you may use methods that operate many items at once(`push_slice`/`push_iter`, `pop_slice`, etc.).
Caching endpoints also avoid repeatedly fetching the opposite endpoint's index when progress is possible.
All completed operations publish their index updates immediately, including each step of `pop_iter`.

`skip` and `clear` take constant time for items without destructors, such as `u8`.
Items that need destruction are dropped individually, with each slot kept occupied until its destructor finishes.

For single-threaded usage `LocalRb` is recommended because it is slightly faster than `SharedRb` due to absence of CPU cache synchronization.

## Migrating from deferred publication

Deferred publication in frozen endpoints and `PopIter` could lead to double drops if they were forgotten with `core::mem::forget`.
Publication no longer depends on running their destructors.

`Frozen`, `FrozenProd`, `FrozenCons`, and `freeze()` are deprecated compatibility APIs. Their removal is reserved for a future breaking release.
Existing calls remain available, but code relying on delayed visibility or rollback must be updated:

+ Use `CachingProd` and `CachingCons` directly and remove calls to `freeze`, `commit`, `fetch`, and `sync`.
  Operations publish their changes immediately and fetch the opposite endpoint's progress as needed.
+ `FrozenProd::discard` is now a no-op. Stage items outside the ring buffer if they may need to be discarded, and insert them only when ready to publish.
+ `PopIter` releases each slot as soon as it yields the item. Its `commit` method is deprecated and does nothing.
  It still yields only the items present when it was created. Use `pop_slice` to batch removals.

## Examples

### Simple

```rust
use ringbuf::{traits::*, HeapRb};

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
```

### No heap

```rust
use ringbuf::{traits::*, StaticRb};

const RB_SIZE: usize = 1;
let mut rb = StaticRb::<i32, RB_SIZE>::default();
let (mut prod, mut cons) = rb.split_ref();

assert_eq!(prod.try_push(123), Ok(()));
assert_eq!(prod.try_push(321), Err(321));

assert_eq!(cons.try_pop(), Some(123));
assert_eq!(cons.try_pop(), None);
```

## Overwrite

Ring buffer can be used in overwriting mode when insertion overwrites the oldest element if the buffer is full.

```rust
use ringbuf::{traits::*, HeapRb};

let mut rb = HeapRb::<i32>::new(2);

assert_eq!(rb.push_overwrite(0), None);
assert_eq!(rb.push_overwrite(1), None);
assert_eq!(rb.push_overwrite(2), Some(0));

assert_eq!(rb.try_pop(), Some(1));
assert_eq!(rb.try_pop(), Some(2));
assert_eq!(rb.try_pop(), None);
```

Note that `push_overwrite` requires exclusive access to the ring buffer
so to perform it concurrently you need to guard the ring buffer with mutex or some other lock.

## Derived crates

+ [`ringbuf-blocking`](https://crates.io/crates/ringbuf-blocking)
+ [`async-ringbuf`](https://crates.io/crates/async-ringbuf)

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
