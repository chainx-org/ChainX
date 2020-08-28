//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0-rc6

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub struct WeightInfo;
impl xpallet_assets_registrar::WeightInfo for WeightInfo {
    fn register() -> Weight {
        (331853000 as Weight)
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn deregister() -> Weight {
        (177795000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(4 as Weight))
    }
    fn recover() -> Weight {
        (238223000 as Weight)
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn update_asset_info() -> Weight {
        (93039000 as Weight)
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
