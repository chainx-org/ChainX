// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]

use sp_std::collections::btree_map::BTreeMap;

use codec::Codec;

pub use chainx_primitives::AssetId;
pub use xpallet_assets::{AssetType, TotalAssetInfo};

sp_api::decl_runtime_apis! {
    pub trait XAssetsApi<AccountId, Balance>
    where
        AccountId: Codec,
        Balance: Codec,
    {
        fn assets_for_account(who: AccountId) -> BTreeMap<AssetId, BTreeMap<AssetType, Balance>>;

        fn assets() -> BTreeMap<AssetId, TotalAssetInfo<Balance>>;
    }
}
