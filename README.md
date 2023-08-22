# ringbuf

[![Crates.io][crates_badge]][crates]
[![Docs.rs][docs_badge]][docs]
[![Gitlab CI][gitlab_badge]][gitlab]
[![License][license_badge]][license]

[crates_badge]: https://img.shields.io/crates/v/ringbuf.svg
[docs_badge]: https://docs.rs/ringbuf/badge.svg
[gitlab_badge]: https://gitlab.com/agerasev/ringbuf/badges/master/pipeline.svg
[license_badge]: https://img.shields.io/crates/l/ringbuf.svg

[crates]: https://crates.io/crates/ringbuf
[docs]: https://docs.rs/ringbuf
[gitlab]: https://gitlab.com/agerasev/ringbuf/-/pipelines?scope=branches&ref=master
[license]: #license

Lock-free SPSC FIFO ring buffer with direct access to inner data.

## Features

+ Lock-free operations - they succeed or fail immediately without blocking or waiting.
+ Arbitrary item type (not only `Copy`).
+ Items can be inserted and removed one by one or many at once.
+ Thread-safe direct access to the internal ring buffer memory.
+ `Read` and `Write` implementation.
+ Overwriting mode support.
+ Can be used without `std` and even without `alloc` (using only statically-allocated memory).
+ [Experimental `async`/`.await` support](./async).

## Usage

At first you need to create the ring buffer itself. `HeapRb` is recommended but you may [choose another one](#types).

After the ring buffer is created it may be splitted into pair of `Producer` and `Consumer`.
`Producer` is used to insert items to the ring buffer, `Consumer` - to remove items from it.
For `SharedRb` and its derivatives they can be used in different threads.

## Types

There are several types of ring buffers provided:

+ `LocalRb`. Only for single-threaded use.
+ `SharedRb`. Can be shared between threads. Its derivatives:
  + `HeapRb`. Contents are stored in dynamic memory. *Recommended for use in most cases.*
  + `StaticRb`. Contents can be stored in statically-allocated memory.

## Performance

`SharedRb` needs to synchronize CPU cache between CPU cores. This synchronization has some overhead.
To avoid multiple unnecessary synchronizations you may use postponed mode of operation (see description for `Producer` and `Consumer`)
or methods that operates many items at once (`Producer::push_slice`/`Producer::push_iter`, `Consumer::pop_slice`, etc.).

For single-threaded usage `LocalRb` is recommended because it is faster than `SharedRb` due to absence of CPU cache synchronization.

### Benchmarks

You may see typical performance of different methods in benchmarks:

```bash
cargo +nightly bench --features bench
```

Nightly toolchain is required.

## Examples

### Simple

```rust
use ringbuf::HeapRb;

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

### No heap

```rust
use ringbuf::StaticRb;

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

## Overwrite

Ring buffer can be used in overwriting mode when insertion overwrites the latest element if the buffer is full.

```rust
use ringbuf::{HeapRb, Rb};

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

Note that [`push_overwrite`](`Rb::push_overwrite`) requires exclusive access to the ring buffer
so to perform it concurrently you need to guard the ring buffer with [`Mutex`](`std::sync::Mutex`) or some other lock.

## `async`/`.await`

There is an experimental crate [`async-ringbuf`](https://gitlab.com/agerasev/async-ringbuf)
which is built on top of `ringbuf` and implements asynchronous ring buffer operations.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
