//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0-rc6

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};

pub struct WeightInfo;
impl xpallet_assets::WeightInfo for WeightInfo {
	fn transfer() -> Weight {
		(1288918000 as Weight)
			.saturating_add(DbWeight::get().reads(12 as Weight))
			.saturating_add(DbWeight::get().writes(8 as Weight))
	}
	fn force_transfer() -> Weight {
		(1145034000 as Weight)
			.saturating_add(DbWeight::get().reads(12 as Weight))
			.saturating_add(DbWeight::get().writes(8 as Weight))
	}
	fn set_balance(n: u32, ) -> Weight {
		(1478617000 as Weight)
			.saturating_add((27703000 as Weight).saturating_mul(n as Weight))
			.saturating_add(DbWeight::get().reads(7 as Weight))
			.saturating_add(DbWeight::get().writes(5 as Weight))
	}
	fn set_asset_limit(n: u32, ) -> Weight {
		(74906000 as Weight)
			.saturating_add((5136000 as Weight).saturating_mul(n as Weight))
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
}
