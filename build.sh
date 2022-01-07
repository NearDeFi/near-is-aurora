#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e
pushd "$(dirname $0)"

RUSTFLAGS='-C link-arg=-s' cargo build --all --target wasm32-unknown-unknown --release
mkdir -p ./res
cp $TARGET/wasm32-unknown-unknown/release/contract.wasm ./res/

popd
