// xpallet_mining_asset
pub trait WeightInfo {
	fn claim() -> Weight;
	fn set_claim_staking_requirement(c: u32, ) -> Weight;
	fn set_claim_frequency_limit(c: u32, ) -> Weight;
	fn set_asset_power(c: u32, ) -> Weight;
}
