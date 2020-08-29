// xpallet_assets
pub trait WeightInfo {
	fn transfer() -> Weight;
	fn force_transfer() -> Weight;
	fn set_balance(n: u32, ) -> Weight;
	fn set_asset_limit(n: u32, ) -> Weight;
}
