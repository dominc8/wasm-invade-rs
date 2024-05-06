#!/bin/sh

set -eu

TARGET=wasm32-unknown-unknown
BINARY=target/$TARGET/release/invade_rs.wasm

cd ./invade-rs
cargo build --target $TARGET --release
wasm-strip $BINARY
wasm-opt -o ../www/invade_rs.wasm -Oz $BINARY
cd ..
ls -lh www/invade_rs.wasm
