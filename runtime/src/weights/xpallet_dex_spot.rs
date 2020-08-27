//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0-rc6

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub struct WeightInfo;
impl xpallet_dex_spot::WeightInfo for WeightInfo {
    fn put_order() -> Weight {
        (1561152000 as Weight)
            .saturating_add(DbWeight::get().reads(15 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn cancel_order() -> Weight {
        (1283570000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn force_cancel_order() -> Weight {
        (1324347000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn set_handicap() -> Weight {
        (33505000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn set_price_fluctuation() -> Weight {
        (166833000 as Weight)
            .saturating_add(DbWeight::get().reads(4 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn add_trading_pair() -> Weight {
        (293704000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn update_trading_pair() -> Weight {
        (296593000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
}
