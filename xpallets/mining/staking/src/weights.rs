// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_mining_staking
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-11-22, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
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
    fn set_minimum_penalty() -> Weight;
    fn set_sessions_per_era() -> Weight;
}

/// Weights for xpallet_mining_staking using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn register() -> Weight {
        (2_094_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(92_u64))
            .saturating_add(T::DbWeight::get().writes(7_u64))
    }
    fn bond() -> Weight {
        (227_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(9_u64))
            .saturating_add(T::DbWeight::get().writes(5_u64))
    }
    fn unbond() -> Weight {
        (153_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(6_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (120_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
    fn rebond() -> Weight {
        (199_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(10_u64))
            .saturating_add(T::DbWeight::get().writes(5_u64))
    }
    fn claim() -> Weight {
        (155_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(5_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
    fn chill() -> Weight {
        (1_407_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(55_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn validate() -> Weight {
        (25_000_000_u64)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_validator_count() -> Weight {
        (4_000_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_minimum_validator_count() -> Weight {
        (4_000_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_bonding_duration() -> Weight {
        (4_000_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_validator_bonding_duration() -> Weight {
        (4_000_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_minimum_penalty() -> Weight {
        (4_000_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_sessions_per_era() -> Weight {
        (4_000_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn register() -> Weight {
        (2_094_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(92_u64))
            .saturating_add(RocksDbWeight::get().writes(7_u64))
    }
    fn bond() -> Weight {
        (227_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(9_u64))
            .saturating_add(RocksDbWeight::get().writes(5_u64))
    }
    fn unbond() -> Weight {
        (153_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(6_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (120_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(4_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }
    fn rebond() -> Weight {
        (199_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(10_u64))
            .saturating_add(RocksDbWeight::get().writes(5_u64))
    }
    fn claim() -> Weight {
        (155_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(5_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }
    fn chill() -> Weight {
        (1_407_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(55_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn validate() -> Weight {
        (25_000_000_u64)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_validator_count() -> Weight {
        (4_000_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_minimum_validator_count() -> Weight {
        (4_000_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_bonding_duration() -> Weight {
        (4_000_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_validator_bonding_duration() -> Weight {
        (4_000_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_minimum_penalty() -> Weight {
        (4_000_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_sessions_per_era() -> Weight {
        (4_000_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
