#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

use sp_std::collections::btree_map::BTreeMap;

pub use chainx_primitives::{AssetId, Precision};
pub use xrml_assets::{AssetInfo, AssetRestrictions, AssetType, Chain, TotalAssetInfo};

sp_api::decl_runtime_apis! {
    pub trait AssetsApi<AccountId, Balance> where
        AccountId: Codec,
        Balance: Codec,
    {
        fn assets_for_account(who: AccountId) -> BTreeMap<AssetId, BTreeMap<AssetType, Balance>>;

        fn assets() -> BTreeMap<AssetId, TotalAssetInfo<Balance>>;
    }
}
