#!/bin/sh

set -eu

TARGET=wasm32-unknown-unknown
BINARY=target/$TARGET/release/invade_rs.wasm

cargo build --target $TARGET --release
wasm-strip $BINARY
mkdir -p www
wasm-opt -o www/invade_rs.wasm -Oz $BINARY
ls -lh www/invade_rs.wasm
