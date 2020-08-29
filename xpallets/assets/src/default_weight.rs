use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

use crate::WeightInfo;

impl WeightInfo for () {
    fn transfer() -> Weight {
        (850125000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn force_transfer() -> Weight {
        (841515000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn set_balance(n: u32) -> Weight {
        (1225425000 as Weight)
            .saturating_add((342000 as Weight).saturating_mul(n as Weight))
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn set_asset_limit() -> Weight {
        (50328000 as Weight)
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
