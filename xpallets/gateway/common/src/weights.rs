// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_gateway_common
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-12-06, STEPS: 50, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 128

// Executed Command:
// ./target/release/chainx
// benchmark
// --chain=benchmarks
// --steps=50
// --repeat=20
// --pallet=xpallet_gateway_common
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./xpallets/gateway/common/src/weights.rs
// --template=./scripts/xpallet-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for xpallet_gateway_common.
pub trait WeightInfo {
    fn withdraw() -> Weight;
    fn cancel_withdrawal() -> Weight;
    fn setup_trustee() -> Weight;
    fn transition_trustee_session() -> Weight;
    fn set_withdrawal_state() -> Weight;
    fn set_trustee_info_config() -> Weight;
    fn force_set_referral_binding() -> Weight;
    fn change_trustee_transition_duration() -> Weight;
    fn set_trustee_admin() -> Weight;
}

/// Weights for xpallet_gateway_common using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn withdraw() -> Weight {
        (160_353_000_u64)
            .saturating_add(T::DbWeight::get().reads(10_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn cancel_withdrawal() -> Weight {
        (85_879_000_u64)
            .saturating_add(T::DbWeight::get().reads(6_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
    fn setup_trustee() -> Weight {
        (40_304_000_u64)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn transition_trustee_session() -> Weight {
        (1_788_281_000_u64)
            .saturating_add(T::DbWeight::get().reads(8_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn set_withdrawal_state() -> Weight {
        (128_975_000_u64)
            .saturating_add(T::DbWeight::get().reads(11_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn set_trustee_info_config() -> Weight {
        (3_940_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn force_set_referral_binding() -> Weight {
        (20_143_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn change_trustee_transition_duration() -> Weight {
        (2_470_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_trustee_admin() -> Weight {
        (11_566_000_u64)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn withdraw() -> Weight {
        (160_353_000_u64)
            .saturating_add(RocksDbWeight::get().reads(10_u64))
            .saturating_add(RocksDbWeight::get().writes(6_u64))
    }
    fn cancel_withdrawal() -> Weight {
        (85_879_000_u64)
            .saturating_add(RocksDbWeight::get().reads(6_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }
    fn setup_trustee() -> Weight {
        (40_304_000_u64)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn transition_trustee_session() -> Weight {
        (1_788_281_000_u64)
            .saturating_add(RocksDbWeight::get().reads(8_u64))
            .saturating_add(RocksDbWeight::get().writes(6_u64))
    }
    fn set_withdrawal_state() -> Weight {
        (128_975_000_u64)
            .saturating_add(RocksDbWeight::get().reads(11_u64))
            .saturating_add(RocksDbWeight::get().writes(6_u64))
    }
    fn set_trustee_info_config() -> Weight {
        (3_940_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn force_set_referral_binding() -> Weight {
        (20_143_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn change_trustee_transition_duration() -> Weight {
        (2_470_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_trustee_admin() -> Weight {
        (11_566_000_u64)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
