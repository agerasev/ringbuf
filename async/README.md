# async-ringbuf

[![Crates.io][crates_badge]][crates]
[![Docs.rs][docs_badge]][docs]
[![Github Actions][github_badge]][github]
[![Gitlab CI][gitlab_badge]][gitlab]
[![License][license_badge]][license]

[crates_badge]: https://img.shields.io/crates/v/async-ringbuf.svg
[docs_badge]: https://docs.rs/async-ringbuf/badge.svg
[github_badge]: https://github.com/agerasev/ringbuf/actions/workflows/test.yml/badge.svg
[gitlab_badge]: https://gitlab.com/agerasev/ringbuf/badges/master/pipeline.svg
[license_badge]: https://img.shields.io/crates/l/ringbuf.svg

[crates]: https://crates.io/crates/async-ringbuf
[docs]: https://docs.rs/async-ringbuf
[github]: https://github.com/agerasev/ringbuf/actions/workflows/test.yml
[gitlab]: https://gitlab.com/agerasev/ringbuf/-/pipelines?scope=branches&ref=master
[license]: #license

`async`/`.await` SPSC FIFO ring buffer.

# Features

+ Arbitrary item type (not only `Copy`).
+ Items can be inserted and removed one by one or many at once.
+ Thread-safe direct access to the internal ring buffer memory.
+ Different types of buffers and underlying storages.
+ Can be used without `std` and even without `alloc`.
+ Does not depends on any async runtime (like `tokio` and `async-std`).

# Overview

This technically is the original [ringbuf](https://crates.io/crates/ringbuf) with an async synchronization.

`AsyncProducer` and `AsyncConsumer` traits provide `async/await` analogs of lock-free methods of `Producer` and `Consumer` with the ability to wait for certain event (such as appearance item or free slot, completing transfer of all items, or closing the opposite endpoint).

## Examples

### Simple

```rust
use async_ringbuf::{traits::*, AsyncHeapRb};
use futures::{executor::block_on, join};

let rb = AsyncHeapRb::<i32>::new(10);
let (prod, cons) = rb.split();

const N_ITERS: i32 = 100;

join!(
    async move {
        let mut prod = prod;
        for i in 0..N_ITERS {
            prod.push(i).await.unwrap();
        }
    },
    async move {
        let mut cons = cons;
        for i in 0..N_ITERS {
            assert_eq!(cons.pop().await, Some(i));
        }
        assert_eq!(cons.pop().await, None);
    },
);
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
