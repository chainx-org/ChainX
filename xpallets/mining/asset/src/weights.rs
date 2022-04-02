// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_mining_asset
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-03-16, STEPS: 50, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 128

// Executed Command:
// ./target/release/chainx
// benchmark
// --chain=benchmarks
// --steps=50
// --repeat=20
// --pallet=xpallet_mining_asset
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./xpallets/mining/asset/src/weights.rs
// --template=./scripts/xpallet-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for xpallet_mining_asset.
pub trait WeightInfo {
    fn claim() -> Weight;
    fn set_claim_staking_requirement() -> Weight;
    fn set_claim_frequency_limit() -> Weight;
    fn set_asset_power() -> Weight;
}

/// Weights for xpallet_mining_asset using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn claim() -> Weight {
        (166_747_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(12 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn set_claim_staking_requirement() -> Weight {
        (7_188_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_claim_frequency_limit() -> Weight {
        (6_957_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_asset_power() -> Weight {
        (3_593_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn claim() -> Weight {
        (166_747_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(12 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn set_claim_staking_requirement() -> Weight {
        (7_188_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_claim_frequency_limit() -> Weight {
        (6_957_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_asset_power() -> Weight {
        (3_593_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
