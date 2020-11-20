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
impl<T: frame_system::Trait> WeightInfo for SubstrateWeight<T> {
    fn put_order() -> Weight {
        (247_656_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(11 as Weight))
            .saturating_add(T::DbWeight::get().writes(6 as Weight))
    }
    fn cancel_order() -> Weight {
        (218_246_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn force_cancel_order() -> Weight {
        (234_515_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn set_handicap() -> Weight {
        (7_285_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn set_price_fluctuation() -> Weight {
        (31_448_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn add_trading_pair() -> Weight {
        (105_470_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn update_trading_pair() -> Weight {
        (46_907_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn put_order() -> Weight {
        (247_656_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(11 as Weight))
            .saturating_add(RocksDbWeight::get().writes(6 as Weight))
    }
    fn cancel_order() -> Weight {
        (218_246_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn force_cancel_order() -> Weight {
        (234_515_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn set_handicap() -> Weight {
        (7_285_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn set_price_fluctuation() -> Weight {
        (31_448_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn add_trading_pair() -> Weight {
        (105_470_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(2 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn update_trading_pair() -> Weight {
        (46_907_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
