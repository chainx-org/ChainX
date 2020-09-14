// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use chainx_primitives::AssetId;
use codec::Codec;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;
use xpallet_mining_asset::{MiningAssetInfo, RpcMinerLedger};
use xpallet_support::RpcBalance;

sp_api::decl_runtime_apis! {
    /// The API to query mining asset info.
    pub trait XMiningAssetApi<AccountId, Balance, BlockNumber> where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
    {
        /// Get overall information about all mining assets.
        fn mining_assets() -> Vec<MiningAssetInfo<AccountId, RpcBalance<Balance>, BlockNumber>>;

        /// Get the asset mining dividends info given the asset miner AccountId.
        fn mining_dividend(who: AccountId) -> BTreeMap<AssetId, RpcBalance<Balance>>;

        /// Get the mining ledger details given the asset miner AccountId.
        fn miner_ledger(who: AccountId) -> BTreeMap<AssetId, RpcMinerLedger<BlockNumber>>;
    }
}
