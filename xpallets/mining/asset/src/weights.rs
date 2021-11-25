// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_mining_asset
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-11-20, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
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
        (231_026_000_u64)
            .saturating_add(T::DbWeight::get().reads(12_u64))
            .saturating_add(T::DbWeight::get().writes(5_u64))
    }
    fn set_claim_staking_requirement() -> Weight {
        (11_222_000_u64)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_claim_frequency_limit() -> Weight {
        (11_103_000_u64)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_asset_power() -> Weight {
        (5_538_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn claim() -> Weight {
        (231_026_000_u64)
            .saturating_add(RocksDbWeight::get().reads(12_u64))
            .saturating_add(RocksDbWeight::get().writes(5_u64))
    }
    fn set_claim_staking_requirement() -> Weight {
        (11_222_000_u64)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_claim_frequency_limit() -> Weight {
        (11_103_000_u64)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_asset_power() -> Weight {
        (5_538_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
