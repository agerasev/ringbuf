#[cfg(feature = "alloc")]
use crate::storage::Heap;
use crate::{
    index::{LocalIndex, SharedIndex},
    storage::Static,
    Cons, Prod, Rb,
};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;

pub type LocalRb<S> = Rb<S, LocalIndex, LocalIndex>;

/// Ring buffer that can be shared between threads.
///
/// Note that there is no explicit requirement of `T: Send`. Instead [`Rb`] will work just fine even with `T: !Send`
/// until you try to send its [`Prod`] or [`Cons`] to another thread.
#[cfg_attr(
    feature = "std",
    doc = r##"
```
use std::thread;
use ringbuf::{SharedRb, storage::Heap, traits::*};

let rb = SharedRb::<Heap<i32>>::new(256);
let (mut prod, mut cons) = rb.split_arc();
thread::spawn(move || {
    prod.try_push(123).unwrap();
})
.join();
thread::spawn(move || {
    assert_eq!(cons.try_pop().unwrap(), 123);
})
.join();
```
"##
)]
pub type SharedRb<S> = Rb<S, SharedIndex, SharedIndex>;

/// Stack-allocated ring buffer with static capacity.
///
/// *Capacity (`N`) must be greater that zero.*
pub type StaticRb<T, const N: usize> = SharedRb<Static<T, N>>;

/// Alias for [`StaticRb`] producer.
pub type StaticProd<'a, T, const N: usize> = Prod<&'a StaticRb<T, N>>;

/// Alias for [`StaticRb`] consumer.
pub type StaticCons<'a, T, const N: usize> = Cons<&'a StaticRb<T, N>>;

/// Heap-allocated ring buffer.
#[cfg(feature = "alloc")]
pub type HeapRb<T> = SharedRb<Heap<T>>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] producer.
pub type HeapProd<T> = Prod<Arc<HeapRb<T>>>;

#[cfg(feature = "alloc")]
/// Alias for [`HeapRb`] consumer.
pub type HeapCons<T> = Cons<Arc<HeapRb<T>>>;
