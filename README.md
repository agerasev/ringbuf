# ringbuf

[![Crates.io][crates_badge]][crates]
[![Docs.rs][docs_badge]][docs]
[![Github Actions][github_badge]][github]
[![License][license_badge]][license]

[crates_badge]: https://img.shields.io/crates/v/ringbuf.svg
[docs_badge]: https://docs.rs/ringbuf/badge.svg
[github_badge]: https://github.com/agerasev/ringbuf/actions/workflows/test.yml/badge.svg
[license_badge]: https://img.shields.io/crates/l/ringbuf.svg

[crates]: https://crates.io/crates/ringbuf
[docs]: https://docs.rs/ringbuf
[github]: https://github.com/agerasev/ringbuf/actions/workflows/test.yml
[license]: #license

Lock-free single-producer single-consumer (SPSC) FIFO ring buffer with direct access to inner data.

# Overview

The initial thing you probably want to start with is to choose some implementation of the `Rb` trait representing ring buffer itself.

Implementations of the `Rb` trait:

+ `LocalRb`. Only for single-threaded use.
+ `SharedRb`. Can be shared between threads.
  + `HeapRb`. Contents are stored in dynamic memory. *Recommended for use in most cases.*
  + `StaticRb`. Contents can be stored in statically-allocated memory.

Ring buffer can be splitted into pair of `Producer` and `Consumer`.

`Producer` and `Consumer` are used to append/remove items to/from the ring buffer accordingly. For `SharedRb` they can be safely sent between threads.
Operations with `Producer` and `Consumer` are lock-free - they succeed or fail immediately without blocking or waiting.

Elements can be effectively appended/removed one by one or many at once.
Also data could be loaded/stored directly into/from `Read`/`Write` instances.
And finally, there are `unsafe` methods allowing thread-safe direct access to the inner memory being appended/removed.

The crate can be used without `std` and even without `alloc` using only statically-allocated memory.

When building with nightly toolchain it is possible to run benchmarks via `cargo bench --features bench`.

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

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
