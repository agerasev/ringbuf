# Experimental 0.6 design

This document records the agreed design and the implementation milestones. The
experiment is developed on the existing `v6` branch, with a checked commit at
each milestone. Public traits remain implementable downstream.

## Representation and ownership

`Rb<S, I, M>` has explicit storage, index, and marker parameters. `SharedRb` and
`LocalRb` are aliases for one implementation. The backing RB stores all three
indices; the atomic index policy synchronizes all three. In logical unwrapped
coordinates, at operation boundaries:

```text
read_released <= read_claimed <= write_published <= read_released + capacity
```

Indices wrap modulo twice the capacity. Capacity is in `1..=usize::MAX / 2`.
The producer can reuse only space before `read_released`. The RB owns live
values in `[read_claimed, write_published)`. A deferred consumer owns its claimed
range, including any values it has already moved out. Unpublished writes belong
to the producer and are deliberately not tracked by an additional shared index.

`retained_len` includes claimed but unreleased slots; `queued_len` excludes them.
Concurrent observations are advisory, not a coherent snapshot. Unsafe operations
must use the exclusive endpoint's valid ranges, not arbitrary observer counts.

## Endpoints

The `endpoint` module replaces `wrap`: `Direct`, `Cached`, and `Deferred`, with
producer/consumer aliases. `Producer`, `Consumer`, and `Observer` are safe APIs;
raw implementation contracts that safe operations rely on are unsafe traits.
`RbHandle` permits caller-provided owning and borrowed backing handles.

Deferred endpoints support owning and borrowed forms using the same machinery.
`commit(&mut self)` publishes progress; producer-only `discard(&mut self)` drops
pending writes without publishing. `fetch(&mut self)` acquires all newly
available slots without waiting and returns their count. `sync` commits then
fetches. `undo_push` retrieves the newest pending write; `undo_pop(value)` restores
the newest removed slot. Undo cannot cross the latest commit boundary.

Normal destruction commits, including during unwinding. Consumer finalization
also hands its untouched claimed suffix back to the RB. Discard must remove the
entire pending range from committable state before running any item destructor.
The backing memory cannot be reused while a destructor or a borrowed view still
accesses it. Notification callbacks run only after ownership is consistent.

Forgetting a deferred producer can leak unpublished values. Forgetting a deferred
consumer can leak its entire unread reservation. On the next ordinary consumer
access, setting `read_released = read_claimed` reclaims an abandoned reservation
without inspecting or dropping its contents. Fetching through an active deferred
consumer must never run this recovery path. Final RB destruction drops only its
own live range, so forgetting cannot cause double destruction.

## Markers and waiting

One marker-policy parameter covers both endpoints. No-marker policies allow only
owned or exclusive-borrow splitting (or explicitly unsafe acquisition). Checked
policies also permit fallible acquisition through shared handles. Tracking is
independent of index synchronization. Both endpoints are established before
`split` returns.

Any absence of the opposite endpoint means closed, including never acquired.
There is no permanent closed-state bit. Basic operations ignore presence.
Waiting consumers drain published values before reporting closure. Reacquisition
does not restart completed waits. Publish final progress before removing presence.
Async and blocking policies signal publication, slot release, and endpoint drop.
Waiting must recheck after registration/arming; threshold waits need progress
notifications beyond empty/nonempty transitions.

## Bulk operations and allocation

Partial transfers return the completed count. Exact nonwaiting transfers either
succeed in full or leave the queue unchanged. Streaming waiting transfers report
partial progress on closure/timeout and document cancellation. Exact reservations
larger than capacity fail immediately. Fixed-size array transfers support non-Copy
items. Safe initialization helpers belong on endpoints, not a restricted public
chunk type.

Keep `new` and `try_new`; the latter returns capacity and storage-allocation errors.
Default owned splitting may retain standard `Arc` allocation behavior. Users can
provide an alternative backing handle. Borrowed splitting does not allocate.

## Milestones

- [x] 1. Contracts, executable ownership model, and baseline checks.
- [x] 2. One generic core, index policies, and raw/safe interfaces.
- [x] 3. Configurable ownership markers and handle acquisition.
- [x] 4. Extensible deferred endpoints, undo, and forget/panic recovery.
- [x] 5. Exact/partial bulk operations and fallible construction.
- [ ] 6. Async/blocking marker backends and consistent waiting APIs.
- [ ] 7. Feature/target checks, Miri, benchmarks, examples, and migration notes.

Milestone 1 validation: `cargo test --workspace --offline` passes (100 unit
tests and 5 doctests); `cargo test --offline --test ownership_model` passes
all three exhaustive small-capacity cursor models.

Milestone 2 validation: workspace tests, local-policy tests, and a no-default-feature
build pass. Data views now require an exclusive borrow, preventing shared backing
handles from aliasing an active endpoint's data. Public raw and delegation traits
have explicit unsafe implementation contracts.

Milestone 3 validation: workspace tests pass, including untracked exclusive
recovery, partial-acquisition rollback, custom handles, and atomic indices with
no markers across threads. Presence queries are a separate capability.

Milestone 4 validation: workspace tests and all seven deferred ownership tests
pass under Miri. Miri uses a writable cache in `/tmp`; leak checking is disabled
for tests that deliberately exercise the specified `mem::forget` leaks.

Milestone 5 validation: workspace tests pass; exact-transfer and allocation-error
tests, including non-Copy arrays and initializer panic recovery, pass under Miri.
