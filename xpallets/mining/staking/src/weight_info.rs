// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::weights::Weight;

pub trait WeightInfo {
    fn register() -> Weight;
    fn bond() -> Weight;
    fn unbond() -> Weight;
    fn unlock_unbonded_withdrawal() -> Weight;
    fn rebond() -> Weight;
    fn claim() -> Weight;
    fn chill() -> Weight;
    fn validate() -> Weight;
    fn set_validator_count() -> Weight;
    fn set_minimum_validator_count() -> Weight;
    fn set_bonding_duration() -> Weight;
    fn set_validator_bonding_duration() -> Weight;
}

impl WeightInfo for () {
    fn register() -> Weight {
        1_000_000_000
    }
    fn bond() -> Weight {
        1_000_000_000
    }
    fn unbond() -> Weight {
        1_000_000_000
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        1_000_000_000
    }
    fn rebond() -> Weight {
        1_000_000_000
    }
    fn claim() -> Weight {
        1_000_000_000
    }
    fn chill() -> Weight {
        1_000_000_000
    }
    fn validate() -> Weight {
        1_000_000_000
    }
    fn set_validator_count() -> Weight {
        1_000_000_000
    }
    fn set_minimum_validator_count() -> Weight {
        1_000_000_000
    }
    fn set_bonding_duration() -> Weight {
        1_000_000_000
    }
    fn set_validator_bonding_duration() -> Weight {
        1_000_000_000
    }
}
