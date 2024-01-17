#!/bin/sh

cargo test && \
cargo test --features test_local && \
cargo check --no-default-features --features alloc && \
cargo check --no-default-features && \
cd async && \
cargo test && \
cargo test --no-default-features --features alloc && \
cargo check --no-default-features --features alloc && \
cargo check --no-default-features && \
cd ../blocking && \
cargo test && \
cargo check --no-default-features --features alloc && \
cargo check --no-default-features && \
echo "Done!"
