// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_gateway_bitcoin
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-12-27, STEPS: 50, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 128

// Executed Command:
// ./target/release/sherpax
// benchmark
// --chain=benchmarks
// --steps=50
// --repeat=20
// --pallet=xpallet_gateway_bitcoin
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./xpallets/gateway/bitcoin/src/weights.rs
// --template=./scripts/xpallet-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for xpallet_gateway_bitcoin.
pub trait WeightInfo {
    fn push_header() -> Weight;
    fn push_transaction() -> Weight;
    fn create_taproot_withdraw_tx() -> Weight;
    fn set_best_index() -> Weight;
    fn set_confirmed_index() -> Weight;
    fn remove_pending() -> Weight;
    fn remove_proposal() -> Weight;
    fn set_btc_withdrawal_fee() -> Weight;
    fn set_btc_deposit_limit() -> Weight;
}

/// Weights for xpallet_gateway_bitcoin using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn push_header() -> Weight {
        (100_290_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(10 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn push_transaction() -> Weight {
        (225_626_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(19 as Weight))
            .saturating_add(T::DbWeight::get().writes(9 as Weight))
    }
    fn create_taproot_withdraw_tx() -> Weight {
        (151_224_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(13 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn set_best_index() -> Weight {
        (5_701_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_confirmed_index() -> Weight {
        (5_649_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn remove_pending() -> Weight {
        (277_254_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn remove_proposal() -> Weight {
        (4_547_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_btc_withdrawal_fee() -> Weight {
        (4_730_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_btc_deposit_limit() -> Weight {
        (2_638_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn push_header() -> Weight {
        (100_290_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(10 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn push_transaction() -> Weight {
        (225_626_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(19 as Weight))
            .saturating_add(RocksDbWeight::get().writes(9 as Weight))
    }
    fn create_taproot_withdraw_tx() -> Weight {
        (151_224_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(13 as Weight))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
    }
    fn set_best_index() -> Weight {
        (5_701_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_confirmed_index() -> Weight {
        (5_649_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn remove_pending() -> Weight {
        (277_254_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn remove_proposal() -> Weight {
        (4_547_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_btc_withdrawal_fee() -> Weight {
        (4_730_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_btc_deposit_limit() -> Weight {
        (2_638_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
