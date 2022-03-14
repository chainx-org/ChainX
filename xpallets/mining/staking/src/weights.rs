// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_mining_staking
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-03-14, STEPS: 50, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
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
        (1_369_235_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(149 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn bond() -> Weight {
        (117_824_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(9 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn unbond() -> Weight {
        (87_946_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (81_309_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn rebond() -> Weight {
        (117_985_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(10 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn claim() -> Weight {
        (101_040_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn chill() -> Weight {
        (1_127_182_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(95 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn validate() -> Weight {
        (24_262_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_validator_count() -> Weight {
        (2_424_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_minimum_validator_count() -> Weight {
        (2_450_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_bonding_duration() -> Weight {
        (2_396_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_validator_bonding_duration() -> Weight {
        (2_422_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_minimum_penalty() -> Weight {
        (2_472_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_sessions_per_era() -> Weight {
        (2_455_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn register() -> Weight {
        (1_369_235_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(149 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn bond() -> Weight {
        (117_824_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn unbond() -> Weight {
        (87_946_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (81_309_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(4 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn rebond() -> Weight {
        (117_985_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(10 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn claim() -> Weight {
        (101_040_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(5 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn chill() -> Weight {
        (1_127_182_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(95 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn validate() -> Weight {
        (24_262_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_validator_count() -> Weight {
        (2_424_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_minimum_validator_count() -> Weight {
        (2_450_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_bonding_duration() -> Weight {
        (2_396_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_validator_bonding_duration() -> Weight {
        (2_422_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_minimum_penalty() -> Weight {
        (2_472_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_sessions_per_era() -> Weight {
        (2_455_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
