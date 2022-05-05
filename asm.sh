#!/usr/bin/env bash

mkdir -p asm

cargo +nightly clean
cargo +nightly build --release

rm -f asm/$1.asm
cargo +nightly asm ringbuf::asm::push --no-color >> asm/$1.asm
echo >> asm/$1.asm
cargo +nightly asm ringbuf::asm::pop --no-color >> asm/$1.asm

rm -f asm/$1.ll
cargo +nightly llvm-ir ringbuf::asm::push --no-color >> asm/$1.ll
echo >> asm/$1.ll
cargo +nightly llvm-ir ringbuf::asm::pop --no-color >> asm/$1.ll
