// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_mining_staking
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-11-20, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 128

// Executed Command:
// ./target/release/chainx
// benchmark
// --chain=benchmarks
// --steps=50
// --repeat=20
// --pallet=xpallet_mining_staking
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./xpallets/mining/staking/src/weights.rs
// --template=./scripts/xpallet-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for xpallet_mining_staking.
pub trait WeightInfo {
    fn register() -> Weight;
    fn bond() -> Weight;
    fn unbond() -> Weight;
    fn unlock_unbonded_withdrawal() -> Weight;
    fn rebond() -> Weight;
    fn claim() -> Weight;
    fn chill() -> Weight;
    fn validate() -> Weight;
    fn set_validator_count() -> Weight;
    fn set_minimum_validator_count() -> Weight;
    fn set_bonding_duration() -> Weight;
    fn set_validator_bonding_duration() -> Weight;
}

/// Weights for xpallet_mining_staking using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Trait> WeightInfo for SubstrateWeight<T> {
    fn register() -> Weight {
        (1_612_615_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(92 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn bond() -> Weight {
        (214_417_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(9 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn unbond() -> Weight {
        (163_351_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (134_368_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn rebond() -> Weight {
        (220_979_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(10 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn claim() -> Weight {
        (172_413_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn chill() -> Weight {
        (1_284_578_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(55 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn validate() -> Weight {
        (38_326_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_validator_count() -> Weight {
        (4_253_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_minimum_validator_count() -> Weight {
        (4_190_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_bonding_duration() -> Weight {
        (3_922_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_validator_bonding_duration() -> Weight {
        (3_852_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn register() -> Weight {
        (1_612_615_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(92 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn bond() -> Weight {
        (214_417_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn unbond() -> Weight {
        (163_351_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (134_368_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(4 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn rebond() -> Weight {
        (220_979_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(10 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn claim() -> Weight {
        (172_413_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(5 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn chill() -> Weight {
        (1_284_578_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(55 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn validate() -> Weight {
        (38_326_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_validator_count() -> Weight {
        (4_253_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_minimum_validator_count() -> Weight {
        (4_190_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_bonding_duration() -> Weight {
        (3_922_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_validator_bonding_duration() -> Weight {
        (3_852_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
