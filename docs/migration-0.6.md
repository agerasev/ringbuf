# Experimental 0.6 migration

This branch is an implementation experiment, not a release or a claim of a
completed concurrency audit. Package versions remain unchanged for now.

## Types and naming

| Previous spelling | Preferred spelling |
| --- | --- |
| Separate `LocalRb<S>` / `SharedRb<S>` implementations | Aliases of `Rb<S, I, M>` |
| `wrap` / `Wrap` | `endpoint` / `Endpoint` |
| `Caching` / `CachingProd` / `CachingCons` | `Cached` / `CachedProd` / `CachedCons` |
| `Frozen*` / `freeze()` | `Deferred*` / `into_deferred()` or `defer()` |
| Direct `Prod` / `Cons` | `DirectProd` / `DirectCons`; plain names now select cached endpoints |
| `RbRef`, `rb_ref`, `into_rb_ref` | `RbHandle`, `rb_handle`, `into_rb_handle` |
| `Based` | `Delegate` |
| `storage::Ref` / `storage::Owning` | `BorrowedSlice` / `Inline` |
| `StaticRb` | `ArrayRb` |

Compatibility aliases exist for many type/module names, but this is a breaking
API update. In particular, the previous deprecated Frozen shim published
immediately; its compatibility spelling now has actual deferred semantics.
`DeferredProd<E>` and `DeferredCons<E>` are parameterized by their immediate
endpoint (or an exclusive borrow of it), not directly by the backing handle.

The three primary parameters have no defaults. Storage remains last in the
representation so inline arrays still coerce to dynamically sized slices.
`AtomicIndices` and `LocalIndices` store all three cursors. Marker choices are
`NoMarkers`, `LocalMarkers`, `AtomicMarkers`, `async_ringbuf::AsyncMarkers`, and
`ringbuf_blocking::BlockingMarkers<X>` (with a semaphore implementation `X`).
The policies are independent; async markers also work with local indices.
`EndpointPolicy<S, I>` selects the adapter returned by standard splitting.

## Construction and handles

`HeapRb::new(n)` remains a panicking convenience. `try_new(n)` returns
`CreateError::Capacity` or `CreateError::Allocation`. Heap storage preserves its
allocation capacity separately from logical capacity, avoiding a hidden shrinking
allocation in the fallible constructor. `try_from_storage` returns invalid
storage to the caller. `from_raw_state` and `into_raw_parts` can round-trip all
three cursors. `with_markers` changes policy while owning the RB exclusively.

Default owned splitting uses the standard `Arc` allocation behavior. To choose a
handle, wrap the RB yourself and call `endpoint::try_split(handle)`; `Rc` and
`Arc` implement `RbHandle`, and downstream owning handles can implement its
public unsafe contract. Borrowed splitting allocates nothing. `RbHandle` describes
the lifetime of the RB; `Storage` describes its item allocation.

Checked endpoint constructors offer `try_new` and return `(AcquireError, handle)`
on failure. `try_split` rolls back rights acquired by a failed pair acquisition.
NoMarkers does not implement `Presence`; shared checked acquisition returns
`Untracked`. Owned/exclusive splitting is supported, with explicit unsafe
acquisition available when the caller can prove exclusive endpoint rights.
After a forgotten borrowed endpoint, exclusive access to the RB permits splitting
again even if old tracking flags remain set.

## Deferred endpoints

```rust
use ringbuf::{ArrayRb, traits::*};
let mut rb = ArrayRb::<i32, 4>::default();
let (mut producer, mut consumer) = rb.split_ref();
{
    let mut batch = producer.defer();
    batch.try_push_array([10, 20]).unwrap();
    assert_eq!(consumer.try_pop(), None);
    assert_eq!(batch.undo_push(), Some(20));
    batch.commit();
    assert_eq!(consumer.try_pop(), Some(10));
    batch.fetch();
    batch.try_push(30).unwrap();
} // publishes 30
assert_eq!(consumer.try_pop(), Some(30));
```

- `commit(&mut self)` publishes and establishes the undo boundary.
- `fetch(&mut self)` acquires every currently available peer update without
  waiting, preserves pending work, and returns the additional slot count.
- `sync` commits then fetches.
- `undo_push` returns the last pending write; `discard` drops all pending writes.
- `undo_pop(value)` restores the last removed slot, with no identity restriction
  on the replacement value. Consumers have no discard operation.
- Normal drop commits and returns any unread consumer suffix to the RB.
- Forgetting can leak pending writes or unread claimed values. The next ordinary
  consumer access abandons a forgotten read claim without reading or dropping it.

Undo cannot cross a commit boundary. Discard detaches its entire pending range
before invoking destructors. Panicking initializers retain their initialized
prefix. Panicking callbacks run only after ownership bookkeeping is consistent.
Neither deliberate forgetting nor a panic may cause double destruction.

## Safe and raw APIs

`Observer`, `Producer`, and `Consumer` remain public safe traits. Public unsafe
`RawObserver`, `RawProducer`, `RawConsumer`, and `RawRingBuffer` state the memory,
range, and ownership obligations relied on by their safe defaults. Delegation
contracts and `Endpoint` are unsafe to implement because their base/handle must
stay stable. Nothing is sealed.

Data views now require an exclusive borrow, including `as_slices`, `iter`, peek
operations, and vacant slices. A shared handle to the backing RB therefore cannot
borrow its live data concurrently with an acquired endpoint. Guard views cannot
outlive their deferred endpoint. `pop_iter` follows its endpoint's publication
policy. Advanced code imports the raw traits for explicit cursor updates.

Use `retained_len` for published items plus outstanding reader claims and
`queued_len` for items owned by the RB. Observer statistics are advisory under
concurrency; they are not coherent snapshots. The compatibility `occupied_len`
reports an endpoint's visible range and should not be used to distinguish these
ownership states.

`push_slice`/`pop_slice` still make partial progress. `try_push_slice`/
`try_pop_slice` are immediate and all-or-none. `try_push_array`/`try_pop_array`
move arbitrary non-Copy items; failed pushes return the original array.
`fill_with` initializes available slots safely, retaining the initialized prefix
if the initializer panics. Requests exceeding capacity return `ExactError`.

## Waiting adapters

Async/blocking buffers are aliases of the same core, with notifications in their
marker policy. Direct and cached data operations never check peer presence.
Waiting adapters do: any absent peer means closed, including never acquired.
Consumers drain published items before EOF. Reacquisition changes presence;
completed futures and a completed `Stream` do not restart themselves.
Explicit close is idempotent; an explicitly closed endpoint stays inert even if
another endpoint later acquires the same role.

`wait_vacant` and `wait_occupied` return `Result<(), WaitError>` and reject a count
larger than capacity immediately. A zero count succeeds immediately.
Streaming `push_all`/`pop_all` may exceed capacity and return either the completed
count or `TransferError { completed, reason }`. The old `*_exact` spelling is a
compatibility forwarder to streaming behavior. Iterator transfers report progress;
blocking producers accept a caller-owned `Peekable` to retain unsent values.
Async futures expose their progress for cancellation; an iterator can be retrieved
from its future after manual polling as well.

`async_transfer` never removes an item merely to hold it while awaiting destination
space. Cancellation leaves undelivered items in the source. With `None`, source
closure ends a successful drain; with `Some(n)`, premature closure returns an error.

`pop_into_vec(vec, limit)` appends at most `limit` items and reports allocation
failures without losing the collected prefix. `pop_until_end` is its unbounded
form. Blocking timeouts cover the whole operation; `set_deadline` additionally
supports absolute std deadlines. Async deadlines remain executor-owned: cancel
or select the progress-reporting future using the runtime's timer.
