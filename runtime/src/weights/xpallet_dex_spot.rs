use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};
pub struct WeightForXpalletDexSpot;
impl xpallet_dex_spot::WeightInfo for WeightForXpalletDexSpot {
    fn put_order() -> Weight {
        (1249455000 as Weight)
            .saturating_add(DbWeight::get().reads(15 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn cancel_order() -> Weight {
        (1184239000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
}
