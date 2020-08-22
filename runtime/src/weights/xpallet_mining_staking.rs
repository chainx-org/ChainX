use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};
pub struct WeightForXpalletMiningStaking;
impl xpallet_mining_staking::WeightInfo for WeightForXpalletMiningStaking {
    fn register() -> Weight {
        (330075000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(2 as Weight))
    }
    fn bond() -> Weight {
        (754028000 as Weight)
            .saturating_add(DbWeight::get().reads(13 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn unbond() -> Weight {
        (589326000 as Weight)
            .saturating_add(DbWeight::get().reads(11 as Weight))
            .saturating_add(DbWeight::get().writes(6 as Weight))
    }
    fn unlock_unbonded_withdrawal() -> Weight {
        (528612000 as Weight)
            .saturating_add(DbWeight::get().reads(8 as Weight))
            .saturating_add(DbWeight::get().writes(6 as Weight))
    }
    fn rebond() -> Weight {
        (688491000 as Weight)
            .saturating_add(DbWeight::get().reads(11 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn claim() -> Weight {
        (687880000 as Weight)
            .saturating_add(DbWeight::get().reads(9 as Weight))
            .saturating_add(DbWeight::get().writes(6 as Weight))
    }
    fn chill() -> Weight {
        (417275000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn validate() -> Weight {
        (85952000 as Weight)
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
