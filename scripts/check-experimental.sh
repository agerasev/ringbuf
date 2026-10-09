#!/bin/sh
# Targets must already be installed. Set CARGO_NET_OFFLINE=true for offline runs.
set -eu
cargo test --workspace
cargo test --features test_local
cargo test --features portable-atomic
cargo test --no-default-features --lib
cargo check --no-default-features --features alloc
cargo test -p async-ringbuf --no-default-features --features alloc
cargo check --workspace --no-default-features
cargo check -p ringbuf-blocking --no-default-features --features alloc
for package in ringbuf async-ringbuf ringbuf-blocking; do
    cargo check -p "$package" --target thumbv6m-none-eabi --no-default-features --features alloc,portable-atomic,portable-atomic/critical-section
done
cargo check --workspace --target aarch64-unknown-linux-gnu
cargo check -p ringbuf --target wasm32-unknown-unknown --no-default-features --features alloc
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
cargo fmt --all -- --check
