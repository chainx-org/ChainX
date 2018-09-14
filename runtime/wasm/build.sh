#!/usr/bin/env bash
set -e

cd "$(dirname "${BASH_SOURCE[0]}")"

cargo +nightly build --target=wasm32-unknown-unknown --release
for i in chainx_runtime
do
	wasm-gc target/wasm32-unknown-unknown/release/$i.wasm target/wasm32-unknown-unknown/release/$i.compact.wasm
done
