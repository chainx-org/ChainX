// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Weights for xpallet_dex_spot
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-11-20, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("benchmarks"), DB CACHE: 128

// Executed Command:
// ./target/release/chainx
// benchmark
// --chain=benchmarks
// --steps=50
// --repeat=20
// --pallet=xpallet_dex_spot
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./xpallets/dex/spot/src/weights.rs
// --template=./scripts/xpallet-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for xpallet_dex_spot.
pub trait WeightInfo {
    fn put_order() -> Weight;
    fn cancel_order() -> Weight;
    fn force_cancel_order() -> Weight;
    fn set_handicap() -> Weight;
    fn set_price_fluctuation() -> Weight;
    fn add_trading_pair() -> Weight;
    fn update_trading_pair() -> Weight;
}

/// Weights for xpallet_dex_spot using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn put_order() -> Weight {
        (235_284_000_u64)
            .saturating_add(T::DbWeight::get().reads(11_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn cancel_order() -> Weight {
        (224_571_000_u64)
            .saturating_add(T::DbWeight::get().reads(8_u64))
            .saturating_add(T::DbWeight::get().writes(5_u64))
    }
    fn force_cancel_order() -> Weight {
        (224_649_000_u64)
            .saturating_add(T::DbWeight::get().reads(8_u64))
            .saturating_add(T::DbWeight::get().writes(5_u64))
    }
    fn set_handicap() -> Weight {
        (6_880_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_price_fluctuation() -> Weight {
        (29_885_000_u64).saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn add_trading_pair() -> Weight {
        (57_233_000_u64)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn update_trading_pair() -> Weight {
        (43_873_000_u64)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn put_order() -> Weight {
        (235_284_000_u64)
            .saturating_add(RocksDbWeight::get().reads(11_u64))
            .saturating_add(RocksDbWeight::get().writes(6_u64))
    }
    fn cancel_order() -> Weight {
        (224_571_000_u64)
            .saturating_add(RocksDbWeight::get().reads(8_u64))
            .saturating_add(RocksDbWeight::get().writes(5_u64))
    }
    fn force_cancel_order() -> Weight {
        (224_649_000_u64)
            .saturating_add(RocksDbWeight::get().reads(8_u64))
            .saturating_add(RocksDbWeight::get().writes(5_u64))
    }
    fn set_handicap() -> Weight {
        (6_880_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_price_fluctuation() -> Weight {
        (29_885_000_u64).saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn add_trading_pair() -> Weight {
        (57_233_000_u64)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
    fn update_trading_pair() -> Weight {
        (43_873_000_u64)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
