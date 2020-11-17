#!/usr/bin/env bash

cd "$(dirname "$0")"

cd ..

cargo build --release --features=runtime-benchmarks

bench_run() {
  pallet=$1
  output=$2
  # cargo run --release --features=runtime-benchmarks -- benchmark \
  ./target/release/chainx benchmark \
    --chain=dev \
    --steps=50 \
    --repeat=20 \
    --pallet="$pallet" \
    --extrinsic="*" \
    --execution=wasm \
    --wasm-execution=compiled \
    --heap-pages=4096 \
    --output="$output" \
    --template=./scripts/xpallet-weight-template.hbs
}

bench_run xpallet_mining_asset ./xpallets/mining/asset/src/weights.rs
# bench_run xpallet_mining_staking ./xpallets/mining/staking/src/weights.rs
