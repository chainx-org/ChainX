use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};
pub struct WeightForXpalletMiningAsset;
impl xpallet_mining_asset::WeightInfo for WeightForXpalletMiningAsset {
	fn claim() -> Weight {
		(1119841000 as Weight)
			.saturating_add(DbWeight::get().reads(16 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
}
