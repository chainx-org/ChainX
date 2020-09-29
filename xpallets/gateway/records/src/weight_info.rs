// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn root_deposit() -> Weight;
    fn root_withdraw() -> Weight;
    fn set_withdrawal_state() -> Weight;
    fn set_withdrawal_state_list(u: u32) -> Weight;
}

impl WeightInfo for () {
    fn root_deposit() -> Weight {
        (646702000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(6 as Weight))
    }
    fn root_withdraw() -> Weight {
        (599782000 as Weight)
            .saturating_add(DbWeight::get().reads(9 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn set_withdrawal_state() -> Weight {
        (605412000 as Weight)
            .saturating_add(DbWeight::get().reads(11 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn set_withdrawal_state_list(u: u32) -> Weight {
        (602878000 as Weight)
            .saturating_add((54000 as Weight).saturating_mul(u as Weight))
            .saturating_add(DbWeight::get().reads(11 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
}
