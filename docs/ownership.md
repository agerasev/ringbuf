# Buffer ownership and data views

`LocalRb`, `SharedRb`, and the async/blocking backing buffers implement `Observer`,
`Raw`, and `RingBuffer`. They no longer implement `Producer` or `Consumer`.
Their inherent data-access methods borrow the buffer mutably, including read-only
views. Metadata remains available through shared handles.

Consumer endpoints still support `first`, `as_slices`, and `iter` through `&self`.
Their borrow prevents removal while the view is live; the producer may continue
to append to other slots. Producer endpoints similarly retain shared vacant views.

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
It needs no allocation or runtime ownership checks. The ordinary buffer methods
and its `Read`, `Write`, `fmt::Write`, and owning-iterator support remain available.
Explicit calls such as `Consumer::try_pop(&mut rb)` must use an adapter instead.
The owning buffer iterator is now `rb::IntoIter`; endpoint iterators remain
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
use ringbuf::{StaticRb, alias::Arc, traits::*};
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

A held-flag check at method entry would not protect a view from later acquisition.
An exclusive borrow does:

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

Moving a direct endpoint between threads still permits `Send` items. Sharing that
endpoint requires `Sync` items because shared methods can return `&Item`:

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
