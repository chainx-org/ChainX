//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0-rc6

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub struct WeightInfo;
impl xpallet_mining_asset::WeightInfo for WeightInfo {
    fn claim() -> Weight {
        (1136347000 as Weight)
            .saturating_add(DbWeight::get().reads(16 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    // WARNING! Some components were not used: ["c"]
    fn set_claim_staking_requirement() -> Weight {
        (52052000 as Weight)
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // WARNING! Some components were not used: ["c"]
    fn set_claim_frequency_limit() -> Weight {
        (40637000 as Weight)
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // WARNING! Some components were not used: ["c"]
    fn set_asset_power() -> Weight {
        (19683000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
