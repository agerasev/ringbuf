# Experimental validation — 2026-10-09

## Correctness and portability

The workspace suite passes: 105 library tests, 17 integration tests, and six
doctests (including two compile-fail checks for view lifetimes and NoMarkers
presence queries). The deferred example runs successfully.

`scripts/check-experimental.sh` passes with `CARGO_NET_OFFLINE=true`:

- Default workspace, local indices, and portable-atomic tests.
- Root no-default-feature tests and alloc-only checks.
- Async alloc-only tests and no-std checks for both waiting adapters.
- Embedded `thumbv6m-none-eabi` compilation using portable atomics and critical sections.
- Workspace compilation for AArch64 and alloc-only core compilation for wasm32.
- Clippy across workspace targets with warnings denied, rustdoc with warnings
  denied, and formatting checks.

The cross-target checks compile code; they do not execute it on those targets.

`scripts/miri.sh` passes. Library tests run with leak checking enabled, including
local-index tests. Five existing blocking tests intentionally remain ignored by
Miri because they depend on timing/thread behavior; they pass in native tests.
Bulk, policy, and ownership-model integration tests also run with leak checking.
Only the deferred integration suite disables leak checking, because its explicit
forget scenarios intentionally leak reservations. Drop counters check that moved,
discarded, and retained items are destroyed according to the specified ownership.
A final targeted Miri run also checks the Vec/Box ownership conversions.

Regression coverage includes wraparound, non-Copy items, initializer/destructor
panics, forgotten owned and borrowed endpoints, undo boundaries, failed acquisition
rollback, custom handles, mixed policies, capacity/allocator errors, notification
thresholds, a panicking waker, cancellation with no in-flight transfer item,
closure draining, idempotent close/reacquisition, deadlines, and bounded collection.

## Preliminary single-thread benchmarks

CPU: Intel Core Ultra 7 165H, x86_64, pinned to CPU 0. Compiler: nightly
`5c543b0b8c` (2026-09-29). These are microbenchmarks, not measurements of contention
or inter-core throughput. The baseline had three repetitions; this implementation
had two. Numbers are ranges of the reported point estimates, not confidence intervals.

| Operation | Earlier baseline, ns | Experimental, ns |
| --- | ---: | ---: |
| Local direct push/pop pair, capacity 256 | 1.00–1.02 | 1.07–1.13 |
| Atomic direct push/pop pair, capacity 256 | 1.19–1.22 | 1.47–1.49 |
| Local cached push/pop pair, capacity 256 | 1.29–1.34 | 1.30–1.38 |
| Atomic cached push/pop pair, capacity 256 | 1.29–1.30 | 1.33–1.34 |
| Local cached 100-item push/pop slices | 18.27–19.35 | 16.59–16.67 |
| Atomic cached 100-item push/pop slices | 18.03–18.10 | 20.49–20.86 |

Cached single-item throughput is close to the baseline. Direct operations and
atomic slice transfers regress in this experiment and need profiling before a
release. The added cursor stores, recovery checks, and explicit-close state are
candidates to investigate; these measurements do not establish which is responsible.
There is no claim that every workload improves.

Reproduce the experimental measurement with:

```sh
taskset -c 0 cargo +nightly bench --features bench --bench policies -- --test-threads=1
```

`Heap<u64>` RB sizes are 56 bytes for LocalRb and 384 bytes for SharedRb (previously
48 and 384). `Array<u64, 256>` RB sizes remain 2080 and 2432 bytes. The fallible
heap storage now retains the allocator's capacity separately from logical length.

## Before release

The experimental roadmap is implemented and committed. Release work still needs
an independent review of the unsafe contracts and three-cursor transitions,
performance profiling/tuning, and broader concurrent/inter-core measurements.
Miri and the cursor model cover the exercised cases; they are not an exhaustive
proof of all weak-memory interleavings or of downstream unsafe implementations.
