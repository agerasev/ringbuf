#!/bin/sh
set -eu
cargo +nightly miri test --workspace --lib
cargo +nightly miri test --features test_local --lib
cargo +nightly miri test --test bulk --test ownership_model --test policies
# These tests intentionally leak forgotten reservations; drop counters check ownership.
MIRIFLAGS="${MIRIFLAGS:-} -Zmiri-ignore-leaks" cargo +nightly miri test --test deferred
