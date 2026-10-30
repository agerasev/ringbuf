# Buffer ownership and data views

`LocalRb`, `SharedRb`, and the async/blocking backing buffers implement `Observer`,
`Raw`, and `RingBuffer`. `Producer` and `Consumer` are implemented by endpoints
and the exclusive adapter.
Their inherent data-access methods borrow the buffer mutably, including read-only
views. Metadata is available through shared handles.

Consumer endpoints support `first`, `as_slices`, and `iter` through `&self`.
Their borrow prevents removal while the view is live; the producer may continue
to append to other slots. Producer endpoints provide shared vacant views.

For a generic algorithm accepting an endpoint, use `rb.as_exclusive()`:

```
use ringbuf::{StaticRb, traits::*};
fn pop<C: Consumer<Item = u8>>(consumer: &mut C) -> Option<u8> {
    consumer.try_pop()
}
let mut rb = StaticRb::<u8, 2>::default();
rb.try_push(7).unwrap();
assert_eq!(pop(&mut rb.as_exclusive()), Some(7));
```

The adapter borrows both roles exclusively and does not expose a backing handle.
It needs no allocation or runtime ownership checks. Backing buffers provide
inherent data-access methods and implement `Read`, `Write`, `fmt::Write`, and
`IntoIterator`. To use trait-qualified calls, pass the adapter:
`Consumer::try_pop(&mut rb.as_exclusive())`.
The owning buffer iterator is `rb::IntoIter`; the endpoint iterator is
`traits::consumer::IntoIter`.

## Shared handles cannot borrow data

An endpoint's backing-buffer accessor cannot create another consumer view:

```compile_fail,E0596
use ringbuf::{LocalRb, storage::Array, traits::*, wrap::Wrap};
let mut rb = LocalRb::<Array<String, 2>>::default();
let (mut producer, mut consumer) = rb.split_ref();
producer.try_push("hello".into()).unwrap();
let item = producer.rb().first().unwrap();
drop(consumer.try_pop());
println!("{item}");
```

The same restriction applies through an observer:

```compile_fail,E0596
use ringbuf::{StaticRb, traits::*, wrap::Wrap};
let mut rb = StaticRb::<u8, 2>::default();
let (producer, _consumer) = rb.split_ref();
let observer = producer.observe();
let _items = observer.rb().as_slices();
```

Retaining a handle before splitting does not bypass the restriction:

```compile_fail,E0596
use ringbuf::{LocalRb, storage::Array, traits::*};
use std::rc::Rc;
let rb = Rc::new(LocalRb::<Array<String, 2>>::default());
let (mut producer, mut consumer) = rb.clone().split();
producer.try_push("hello".into()).unwrap();
let item = rb.first().unwrap();
drop(consumer.try_pop());
println!("{item}");
```

```compile_fail,E0596
use ringbuf::{Arc, StaticRb, traits::*};
let rb = Arc::new(StaticRb::<u8, 2>::default());
let (_producer, _consumer) = rb.clone().split();
let _items = rb.as_slices();
```

Neither extracting nor cloning a backing handle grants consumer rights:

```compile_fail,E0596
use ringbuf::{StaticRb, traits::*, wrap::Wrap};
let mut rb = StaticRb::<u8, 2>::default();
let (producer, _consumer) = rb.split_ref();
let handle = producer.into_rb_ref();
let _items = handle.as_slices();
```

```compile_fail,E0596
use ringbuf::{StaticRb, traits::*, wrap::Wrap};
let mut rb = StaticRb::<u8, 2>::default();
let (producer, _consumer) = rb.split_ref();
let _items = producer.rb_ref().as_slices();
```

Vacant memory also requires ownership; `MaybeUninit` does not permit mutation
through another handle while an immutable slice is live:

```compile_fail,E0596
use ringbuf::{StaticRb, traits::*, wrap::Wrap};
let mut rb = StaticRb::<u8, 2>::default();
let (mut producer, consumer) = rb.split_ref();
let slots = consumer.rb().vacant_slices();
producer.try_push(7).unwrap();
println!("{:p}", slots.0.as_ptr());
```

## Views prevent later endpoint acquisition

The exclusive borrow keeps the view valid by preventing endpoint acquisition
for the duration of that view:

```compile_fail,E0502
use ringbuf::{StaticRb, traits::*, wrap::Cons};
let mut rb = StaticRb::<u8, 2>::default();
rb.try_push(7).unwrap();
let item = rb.first().unwrap();
let mut consumer = Cons::new(&rb);
consumer.try_pop();
println!("{item}");
```

The buffer itself cannot satisfy a generic consumer bound:

```compile_fail,E0277
use ringbuf::{StaticRb, traits::Consumer};
fn needs_consumer<C: Consumer>() {}
needs_consumer::<StaticRb<u8, 2>>();
```

Nor can it satisfy a generic producer bound:

```compile_fail,E0277
use ringbuf::{StaticRb, traits::Producer};
fn needs_producer<P: Producer>() {}
needs_producer::<StaticRb<u8, 2>>();
```

## Sharing endpoint views between threads

Moving a direct endpoint between threads requires a `Send` backing handle and
`Send` items. Sharing that endpoint requires a `Sync` backing handle and `Sync`
items because shared methods can return `&Item`:

```compile_fail,E0277
use core::cell::Cell;
use ringbuf::{StaticRb, wrap::Cons};
fn needs_sync<T: Sync>() {}
needs_sync::<Cons<&'static StaticRb<Cell<u8>, 2>>>();
```

```compile_fail,E0277
use core::cell::Cell;
use ringbuf::{StaticRb, wrap::Prod};
fn needs_sync<T: Sync>() {}
needs_sync::<Prod<&'static StaticRb<Cell<u8>, 2>>>();
```

The exclusive adapter follows the same rule:

```compile_fail,E0277
use core::cell::Cell;
use ringbuf::{StaticRb, rb::Exclusive};
fn needs_sync<T: Sync>() {}
needs_sync::<Exclusive<'static, StaticRb<Cell<u8>, 2>>>();
```
