// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_gateway_common
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-05-13, STEPS: 50, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 1024

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
#![allow(clippy::unnecessary_cast)]

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
    fn set_trustee_proxy() -> Weight;
    fn set_trustee_info_config() -> Weight;
    fn set_trustee_admin() -> Weight;
    fn set_trustee_admin_multiply() -> Weight;
    fn claim_trustee_reward() -> Weight;
    fn force_trustee_election() -> Weight;
    fn force_update_trustee() -> Weight;
    fn force_set_referral_binding() -> Weight;
}

/// Weights for xpallet_gateway_common using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn withdraw() -> Weight {
        (148_184_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(9 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn cancel_withdrawal() -> Weight {
        (98_146_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn setup_trustee() -> Weight {
        (101_595_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_proxy() -> Weight {
        (32_483_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_info_config() -> Weight {
        (3_541_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_admin() -> Weight {
        (5_052_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_admin_multiply() -> Weight {
        (3_367_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn claim_trustee_reward() -> Weight {
        (152_350_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn force_trustee_election() -> Weight {
        (25_707_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn force_update_trustee() -> Weight {
        (53_788_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn force_set_referral_binding() -> Weight {
        (19_517_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn withdraw() -> Weight {
        (148_184_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn cancel_withdrawal() -> Weight {
        (98_146_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn setup_trustee() -> Weight {
        (101_595_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_proxy() -> Weight {
        (32_483_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_info_config() -> Weight {
        (3_541_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_admin() -> Weight {
        (5_052_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_trustee_admin_multiply() -> Weight {
        (3_367_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn claim_trustee_reward() -> Weight {
        (152_350_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn force_trustee_election() -> Weight {
        (25_707_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
    }
    fn force_update_trustee() -> Weight {
        (53_788_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn force_set_referral_binding() -> Weight {
        (19_517_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
