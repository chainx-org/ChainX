// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::weights::Weight;

/// Weight information for extrinsics in this pallet.
pub trait WeightInfo {
    fn register() -> Weight;
    fn deregister() -> Weight;
    fn recover() -> Weight;
    fn update_asset_info() -> Weight;
}

impl WeightInfo for () {
    fn register() -> Weight {
        1_000_000_000
    }
    fn deregister() -> Weight {
        1_000_000_000
    }
    fn recover() -> Weight {
        1_000_000_000
    }
    fn update_asset_info() -> Weight {
        1_000_000_000
    }
}
