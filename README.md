# ringbuf

[![Crates.io][crates_badge]][crates]
[![Docs.rs][docs_badge]][docs]
[![Travis CI][travis_badge]][travis]
[![Appveyor][appveyor_badge]][appveyor]
[![Codecov.io][codecov_badge]][codecov]
[![License][license_badge]][license]

[crates_badge]: https://img.shields.io/crates/v/ringbuf.svg
[docs_badge]: https://docs.rs/ringbuf/badge.svg
[travis_badge]: https://api.travis-ci.org/nthend/ringbuf.svg
[appveyor_badge]: https://ci.appveyor.com/api/projects/status/github/nthend/ringbuf?branch=master&svg=true
[codecov_badge]: https://codecov.io/gh/nthend/ringbuf/graphs/badge.svg
[license_badge]: https://img.shields.io/crates/l/ringbuf.svg

[crates]: https://crates.io/crates/ringbuf
[docs]: https://docs.rs/ringbuf
[travis]: https://travis-ci.org/nthend/ringbuf
[appveyor]: https://ci.appveyor.com/project/nthend/ringbuf
[codecov]: https://codecov.io/gh/nthend/ringbuf
[license]: #license

Lock-free single-producer single-consumer ring buffer

## Documentation
+ [`crates.io` version documentation](https://docs.rs/ringbuf)
+ [`master` branch documentation](https://nthend.github.io/ringbuf/target/doc/ringbuf/index.html)

## Examples

### Simple example

```rust
use ringbuf::{RingBuffer, PushError, PopError};

let rb = RingBuffer::<i32>::new(2);
let (mut prod, mut cons) = rb.split();

prod.push(0).unwrap();
prod.push(1).unwrap();
assert_eq!(prod.push(2), Err((PushError::Full, 2)));

assert_eq!(cons.pop().unwrap(), 0);

prod.push(2).unwrap();

assert_eq!(cons.pop().unwrap(), 1);
assert_eq!(cons.pop().unwrap(), 2);
assert_eq!(cons.pop(), Err(PopError::Empty));
```

### Message transfer

This is more complicated example of transfering text message between threads.

```rust
use std::io::{self, Read};
use std::thread;
use std::time::{Duration};

use ringbuf::{RingBuffer, ReadFrom, WriteInto};

let rb = RingBuffer::<u8>::new(10);
let (mut prod, mut cons) = rb.split();

let smsg = "The quick brown fox jumps over the lazy dog";

let pjh = thread::spawn(move || {
    println!("-> sending message: '{}'", smsg);

    let zero = [0 as u8];
    let mut bytes = smsg.as_bytes().chain(&zero[..]);
    loop {
        match prod.read_from(&mut bytes) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                println!("-> {} bytes sent", n);
            },
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
                println!("-> buffer is full, waiting");
                thread::sleep(Duration::from_millis(1));
            },
        }
    }

    println!("-> message sent");
});

let cjh = thread::spawn(move || {
    println!("<- receiving message");

    let mut bytes = Vec::<u8>::new();
    loop {
        match cons.write_into(&mut bytes) {
            Ok(n) => println!("<- {} bytes received", n),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
                if bytes.ends_with(&[0]) {
                    break;
                } else {
                    println!("<- buffer is empty, waiting");
                    thread::sleep(Duration::from_millis(1));
                }
            },
        }
    }

    assert_eq!(bytes.pop().unwrap(), 0);
    let msg = String::from_utf8(bytes).unwrap();
    println!("<- message received: '{}'", msg);

    msg
});

pjh.join().unwrap();
let rmsg = cjh.join().unwrap();

assert_eq!(smsg, rmsg);
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
