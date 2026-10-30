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

The adapter holds both roles exclusively and exposes backing-buffer access only
through a mutable borrow or by consuming the adapter. It needs no allocation or
runtime ownership checks. Backing buffers provide
inherent data-access methods and implement `Read`, `Write`, `fmt::Write`, and
`IntoIterator`. To use trait-qualified calls, pass the adapter:
`Consumer::try_pop(&mut rb.as_exclusive())`.
The owning buffer iterator is `rb::IntoIter`; the endpoint iterator is
`traits::consumer::IntoIter`.

## Owned and borrowed exclusive adapters

`Exclusive<R>` accepts an owned buffer or a mutable borrow through the unsafe
`RbMut` handle contract. `rb.as_exclusive()` constructs `Exclusive<&mut B>`;
`Exclusive::new(rb)` moves the buffer into `Exclusive<B>`.

Use `rb_mut()` for backing-buffer operations such as `push_overwrite`, and
`into_inner()` to recover the owned buffer or mutable borrow:

```
use ringbuf::{StaticRb, rb::Exclusive, traits::*};
let rb = StaticRb::<u8, 2>::from([1, 2]);
let mut exclusive = Exclusive::new(rb);
assert_eq!(exclusive.rb_mut().push_overwrite(3), Some(1));
assert_eq!(exclusive.first(), Some(&2));
let mut rb = exclusive.into_inner();
assert!(rb.pop_iter().eq([2, 3]));
```

Backing-buffer access cannot overlap an adapter's data view:

```compile_fail,E0502
use ringbuf::{StaticRb, rb::Exclusive, traits::*};
let mut exclusive = Exclusive::new(StaticRb::from([String::from("hello")]));
let item = exclusive.first().unwrap();
exclusive.rb_mut().try_pop();
println!("{item}");
```

Recovering the buffer also requires ending all adapter views:

```compile_fail,E0505
use ringbuf::{StaticRb, rb::Exclusive, traits::*};
let mut rb = StaticRb::from([String::from("hello")]);
let exclusive = Exclusive::new(&mut rb);
let item = exclusive.first().unwrap();
let rb = exclusive.into_inner();
rb.try_pop();
println!("{item}");
```

Endpoints created through `rb_mut()` keep the adapter borrowed:

```compile_fail,E0499
use ringbuf::{StaticRb, rb::Exclusive, traits::*};
let mut exclusive = Exclusive::new(StaticRb::<u8, 2>::default());
let (mut producer, _consumer) = exclusive.rb_mut().split_ref();
exclusive.try_push(1).unwrap();
producer.try_push(2).unwrap();
```

The adapter does not implement `RbRef`, `RbMut`, or `RingBuffer`, and it does not
dereference to the backing buffer. In particular, shared access to the adapter
cannot be converted into a backing handle:

```compile_fail,E0277
use ringbuf::{StaticRb, rb::{Exclusive, RbRef}};
fn needs_handle<R: RbRef>() {}
needs_handle::<Exclusive<StaticRb<u8, 2>>>();
```

```compile_fail,E0277
use ringbuf::{StaticRb, rb::Exclusive, traits::RingBuffer};
fn needs_buffer<B: RingBuffer>() {}
needs_buffer::<Exclusive<StaticRb<u8, 2>>>();
```

Shared references and shared ownership do not grant both endpoint roles:

```compile_fail,E0277
use ringbuf::{StaticRb, rb::Exclusive};
let rb = StaticRb::<u8, 2>::default();
let _exclusive = Exclusive::new(&rb);
```

```compile_fail,E0277
use ringbuf::{Arc, StaticRb, rb::{Exclusive, RbRef}};
fn needs_handle<R: RbRef>(_: &R) {}
let rb = Arc::new(StaticRb::<u8, 2>::default());
needs_handle(&rb);
let _exclusive = Exclusive::new(rb);
```

### Implementing buffer handles

`RbRef` provides shared access to one backing buffer. Its unsafe contract requires
stable buffer identity, including across clones when `Clone` is implemented.
`RbMut` extends this contract with exclusive ownership of both roles; merely
providing a method that can sometimes return `&mut B` is insufficient.
The owned buffer types and `&mut B` implement `RbMut`. Shared references, `Arc`,
and `Rc` implement only `RbRef`.

Endpoints accept non-cloneable handles. Cloning an observer and calling
`observe()` require `Clone`, as does constructing a blocking endpoint, which
keeps a separate handle for waiting on its semaphore.

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
needs_sync::<Exclusive<&'static mut StaticRb<Cell<u8>, 2>>>();
```


```compile_fail,E0277
use core::cell::Cell;
use ringbuf::{StaticRb, rb::Exclusive};
fn needs_sync<T: Sync>() {}
needs_sync::<Exclusive<StaticRb<Cell<u8>, 2>>>();
```
