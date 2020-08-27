//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0-rc6

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub struct WeightInfo;
impl xpallet_mining_staking::WeightInfo for WeightInfo {
    fn register() -> Weight {
        (349383000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(2 as Weight))
    }
    fn bond() -> Weight {
        (1099756000 as Weight)
            .saturating_add(DbWeight::get().reads(13 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn unbond() -> Weight {
        (912974000 as Weight)
            .saturating_add(DbWeight::get().reads(10 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (682720000 as Weight)
            .saturating_add(DbWeight::get().reads(8 as Weight))
            .saturating_add(DbWeight::get().writes(6 as Weight))
    }
    fn rebond() -> Weight {
        (917283000 as Weight)
            .saturating_add(DbWeight::get().reads(11 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn claim() -> Weight {
        (927528000 as Weight)
            .saturating_add(DbWeight::get().reads(9 as Weight))
            .saturating_add(DbWeight::get().writes(6 as Weight))
    }
    fn chill() -> Weight {
        (385264000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn validate() -> Weight {
        (103680000 as Weight)
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // WARNING! Some components were not used: ["c"]
    fn set_validator_count() -> Weight {
        (14988000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // WARNING! Some components were not used: ["c"]
    fn set_minimal_validator_count() -> Weight {
        (11813000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // WARNING! Some components were not used: ["c"]
    fn set_bonding_duration() -> Weight {
        (11251000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // WARNING! Some components were not used: ["c"]
    fn set_validator_bonding_duration() -> Weight {
        (12221000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
