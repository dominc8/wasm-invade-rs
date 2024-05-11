#!/bin/sh

set -eu

TARGET=wasm32-unknown-unknown
BINARY=target/$TARGET/release/invade_rs.wasm

cd ./invade-rs
cargo build --target $TARGET --release
wasm-strip $BINARY
wasm-opt -o ../docs/invade_rs.wasm -Oz $BINARY
cd ..
ls -lh docs/invade_rs.wasm
