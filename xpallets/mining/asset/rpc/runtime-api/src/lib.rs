// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]

use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use codec::Codec;

pub use chainx_primitives::AssetId;
pub use xpallet_mining_asset::{AssetLedger, MinerLedger, MiningAssetInfo, MiningWeight};

sp_api::decl_runtime_apis! {
    /// The API to query mining asset info.
    pub trait XMiningAssetApi<AccountId, Balance, MiningWeight, BlockNumber>
    where
        AccountId: Codec,
        Balance: Codec,
        MiningWeight: Codec,
        BlockNumber: Codec,
    {
        /// Get overall information about all mining assets.
        fn mining_assets() -> Vec<MiningAssetInfo<AccountId, Balance, MiningWeight, BlockNumber>>;

        /// Get the asset mining dividends info given the asset miner AccountId.
        fn mining_dividend(who: AccountId) -> BTreeMap<AssetId, Balance>;

        /// Get the mining ledger details given the asset miner AccountId.
        fn miner_ledger(who: AccountId) -> BTreeMap<AssetId, MinerLedger<MiningWeight, BlockNumber>>;
    }
}
