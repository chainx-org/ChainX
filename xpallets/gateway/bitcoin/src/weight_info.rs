// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

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

impl WeightInfo for () {
    fn push_header() -> Weight {
        (1057043000 as Weight)
            .saturating_add(DbWeight::get().reads(14 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    // WARNING! Some components were not used: ["n", "l"]
    fn push_transaction() -> Weight {
        (3828845000 as Weight)
            .saturating_add(DbWeight::get().reads(24 as Weight))
            .saturating_add(DbWeight::get().writes(12 as Weight))
    }
    // WARNING! Some components were not used: ["n", "l"]
    fn create_withdraw_tx() -> Weight {
        (2962661000 as Weight)
            .saturating_add(DbWeight::get().reads(17 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    // WARNING! Some components were not used: ["l"]
    fn sign_withdraw_tx() -> Weight {
        (4275791000 as Weight)
            .saturating_add(DbWeight::get().reads(8 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn set_best_index() -> Weight {
        (26840000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn set_confirmed_index() -> Weight {
        (25080000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn remove_pending() -> Weight {
        (2754215000 as Weight)
            .saturating_add(DbWeight::get().reads(13 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn remove_proposal() -> Weight {
        (16530000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn force_replace_proposal_tx() -> Weight {
        (2962661000 as Weight)
            .saturating_add(DbWeight::get().reads(17 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn set_btc_withdrawal_fee() -> Weight {
        (15960000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn set_btc_deposit_limit() -> Weight {
        (15050000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
