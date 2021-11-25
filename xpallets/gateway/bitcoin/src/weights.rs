// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_gateway_bitcoin
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-11-20, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 128

// Executed Command:
// ./target/release/chainx
// benchmark
// --chain=benchmarks
// --steps=50
// --repeat=20
// --pallet=xpallet_gateway_bitcoin
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./xpallets/gateway/bitcoin/v1/src/weights.rs
// --template=./scripts/xpallet-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for xpallet_gateway_bitcoin.
pub trait WeightInfo {
    fn push_header() -> Weight;
    fn push_transaction() -> Weight;
    fn create_withdraw_tx() -> Weight;
    fn sign_withdraw_tx() -> Weight;
    fn set_best_index() -> Weight;
    fn set_confirmed_index() -> Weight;
    fn remove_pending() -> Weight;
    fn remove_proposal() -> Weight;
    fn force_replace_proposal_tx() -> Weight;
    fn set_btc_withdrawal_fee() -> Weight;
    fn set_btc_deposit_limit() -> Weight;
}

/// Weights for xpallet_gateway_bitcoin using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn push_header() -> Weight {
        (172_185_000_u64)
            .saturating_add(T::DbWeight::get().reads(10_u64))
            .saturating_add(T::DbWeight::get().writes(5_u64))
    }
    fn push_transaction() -> Weight {
        (821_219_000_u64)
            .saturating_add(T::DbWeight::get().reads(21_u64))
            .saturating_add(T::DbWeight::get().writes(10_u64))
    }
    fn create_withdraw_tx() -> Weight {
        (1_022_797_000_u64)
            .saturating_add(T::DbWeight::get().reads(12_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn sign_withdraw_tx() -> Weight {
        (2_141_860_000_u64)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_best_index() -> Weight {
        (5_657_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_confirmed_index() -> Weight {
        (5_234_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn remove_pending() -> Weight {
        (528_851_000_u64)
            .saturating_add(T::DbWeight::get().reads(9_u64))
            .saturating_add(T::DbWeight::get().writes(5_u64))
    }
    fn remove_proposal() -> Weight {
        (4_976_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn force_replace_proposal_tx() -> Weight {
        (143_979_000_u64)
            .saturating_add(T::DbWeight::get().reads(8_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_btc_withdrawal_fee() -> Weight {
        (4_597_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_btc_deposit_limit() -> Weight {
        (4_570_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn push_header() -> Weight {
        (172_185_000_u64)
            .saturating_add(RocksDbWeight::get().reads(10_u64))
            .saturating_add(RocksDbWeight::get().writes(5_u64))
    }
    fn push_transaction() -> Weight {
        (821_219_000_u64)
            .saturating_add(RocksDbWeight::get().reads(21_u64))
            .saturating_add(RocksDbWeight::get().writes(10_u64))
    }
    fn create_withdraw_tx() -> Weight {
        (1_022_797_000_u64)
            .saturating_add(RocksDbWeight::get().reads(12_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
    fn sign_withdraw_tx() -> Weight {
        (2_141_860_000_u64)
            .saturating_add(RocksDbWeight::get().reads(4_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_best_index() -> Weight {
        (5_657_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_confirmed_index() -> Weight {
        (5_234_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn remove_pending() -> Weight {
        (528_851_000_u64)
            .saturating_add(RocksDbWeight::get().reads(9_u64))
            .saturating_add(RocksDbWeight::get().writes(5_u64))
    }
    fn remove_proposal() -> Weight {
        (4_976_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn force_replace_proposal_tx() -> Weight {
        (143_979_000_u64)
            .saturating_add(RocksDbWeight::get().reads(8_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_btc_withdrawal_fee() -> Weight {
        (4_597_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_btc_deposit_limit() -> Weight {
        (4_570_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
