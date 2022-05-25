// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_assets
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-05-13, STEPS: 50, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 1024

// Executed Command:
// ./target/release/chainx
// benchmark
// --chain=benchmarks
// --steps=50
// --repeat=20
// --pallet=xpallet_assets
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./xpallets/assets/src/weights.rs
// --template=./scripts/xpallet-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for xpallet_assets.
pub trait WeightInfo {
    fn transfer() -> Weight;
    fn force_transfer() -> Weight;
    fn set_balance(n: u32) -> Weight;
    fn set_asset_limit() -> Weight;
}

/// Weights for xpallet_assets using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn transfer() -> Weight {
        (160_552_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().writes(6 as Weight))
    }
    fn force_transfer() -> Weight {
        (158_525_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().writes(6 as Weight))
    }
    fn set_balance(_n: u32) -> Weight {
        (170_937_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn set_asset_limit() -> Weight {
        (10_093_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn transfer() -> Weight {
        (160_552_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().writes(6 as Weight))
    }
    fn force_transfer() -> Weight {
        (158_525_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().writes(6 as Weight))
    }
    fn set_balance(_n: u32) -> Weight {
        (170_937_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn set_asset_limit() -> Weight {
        (10_093_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
