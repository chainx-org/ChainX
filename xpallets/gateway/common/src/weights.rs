// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_gateway_common
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-12-07, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
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
    fn transition_trustee_session(u: u32) -> Weight;
    fn set_withdrawal_state() -> Weight;
    fn set_trustee_info_config() -> Weight;
    fn force_set_referral_binding() -> Weight;
    fn change_trustee_transition_duration() -> Weight;
}

/// Weights for xpallet_gateway_common using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn withdraw() -> Weight {
        (226_245_000_u64)
            .saturating_add(T::DbWeight::get().reads(10_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn cancel_withdrawal() -> Weight {
        (130_921_000_u64)
            .saturating_add(T::DbWeight::get().reads(6_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
    fn setup_trustee() -> Weight {
        (40_920_000_u64)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn transition_trustee_session(u: u32) -> Weight {
        (135_412_000_u64)
            .saturating_add((2_000_u64).saturating_mul(u as Weight))
            .saturating_add(T::DbWeight::get().reads(8_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn set_withdrawal_state() -> Weight {
        (217_002_000_u64)
            .saturating_add(T::DbWeight::get().reads(11_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn set_trustee_info_config() -> Weight {
        (6_432_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn force_set_referral_binding() -> Weight {
        (30_667_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn change_trustee_transition_duration() -> Weight {
        (5_657_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn withdraw() -> Weight {
        (226_245_000_u64)
            .saturating_add(RocksDbWeight::get().reads(10_u64))
            .saturating_add(RocksDbWeight::get().writes(6_u64))
    }
    fn cancel_withdrawal() -> Weight {
        (130_921_000_u64)
            .saturating_add(RocksDbWeight::get().reads(6_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }
    fn setup_trustee() -> Weight {
        (40_920_000_u64)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn transition_trustee_session(u: u32) -> Weight {
        (135_412_000_u64)
            .saturating_add((2_000_u64).saturating_mul(u as Weight))
            .saturating_add(RocksDbWeight::get().reads(8_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
    fn set_withdrawal_state() -> Weight {
        (217_002_000_u64)
            .saturating_add(RocksDbWeight::get().reads(11_u64))
            .saturating_add(RocksDbWeight::get().writes(6_u64))
    }
    fn set_trustee_info_config() -> Weight {
        (6_432_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn force_set_referral_binding() -> Weight {
        (30_667_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn change_trustee_transition_duration() -> Weight {
        (5_657_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
