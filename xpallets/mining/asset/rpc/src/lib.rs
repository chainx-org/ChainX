// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! RPC interface for the transaction payment module.

use std::collections::btree_map::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use xp_rpc::runtime_error_into_rpc_err;

use xpallet_support::{RpcBalance, RpcMiningWeight};

use xpallet_mining_asset_rpc_runtime_api::{
    AssetId, AssetLedger, MinerLedger, MiningAssetInfo, XMiningAssetApi as XMiningAssetRuntimeApi,
};

/// XMiningAsset RPC methods.
#[rpc]
pub trait XMiningAssetApi<BlockHash, AccountId, Balance, MiningWeight, BlockNumber>
where
    Balance: Display + FromStr,
    MiningWeight: Display + FromStr,
{
    /// Get overall information about all mining assets.
    #[rpc(name = "xminingasset_getMiningAssets")]
    fn mining_assets(
        &self,
        at: Option<BlockHash>,
    ) -> Result<
        Vec<
            MiningAssetInfo<
                AccountId,
                RpcBalance<Balance>,
                RpcMiningWeight<MiningWeight>,
                BlockNumber,
            >,
        >,
    >;

    /// Get the asset mining dividends info given the asset miner AccountId.
    #[rpc(name = "xminingasset_getDividendByAccount")]
    fn mining_dividend(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AssetId, RpcBalance<Balance>>>;

    /// Get the mining ledger details given the asset miner AccountId.
    #[rpc(name = "xminingasset_getMinerLedgerByAccount")]
    fn miner_ledger(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AssetId, MinerLedger<RpcMiningWeight<MiningWeight>, BlockNumber>>>;
}

/// A struct that implements the [`XMiningAssetApi`].
pub struct XMiningAsset<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> XMiningAsset<C, B> {
    /// Create new `XMiningAsset` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId, Balance, MiningWeight, BlockNumber>
    XMiningAssetApi<<Block as BlockT>::Hash, AccountId, Balance, MiningWeight, BlockNumber>
    for XMiningAsset<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XMiningAssetRuntimeApi<Block, AccountId, Balance, MiningWeight, BlockNumber>,
    AccountId: Codec,
    Balance: Codec + Display + FromStr,
    MiningWeight: Codec + Display + FromStr,
    BlockNumber: Codec,
{
    fn mining_assets(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<
        Vec<
            MiningAssetInfo<
                AccountId,
                RpcBalance<Balance>,
                RpcMiningWeight<MiningWeight>,
                BlockNumber,
            >,
        >,
    > {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .mining_assets(&at)
            .map(|mining_assets| {
                mining_assets
                    .into_iter()
                    .map(|mining_asset| MiningAssetInfo {
                        asset_id: mining_asset.asset_id,
                        mining_power: mining_asset.mining_power,
                        reward_pot: mining_asset.reward_pot,
                        reward_pot_balance: mining_asset.reward_pot_balance.into(),
                        ledger_info: AssetLedger {
                            last_total_mining_weight: mining_asset
                                .ledger_info
                                .last_total_mining_weight
                                .into(),
                            last_total_mining_weight_update: mining_asset
                                .ledger_info
                                .last_total_mining_weight_update,
                        },
                    })
                    .collect::<Vec<_>>()
            })
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn mining_dividend(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, RpcBalance<Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .mining_dividend(&at, who)
            .map(|mining_dividend| {
                mining_dividend
                    .into_iter()
                    .map(|(id, balance)| (id, balance.into()))
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn miner_ledger(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, MinerLedger<RpcMiningWeight<MiningWeight>, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .miner_ledger(&at, who)
            .map(|miner_ledger| {
                miner_ledger
                    .into_iter()
                    .map(|(id, miner_ledger)| {
                        (
                            id,
                            MinerLedger {
                                last_mining_weight: miner_ledger.last_mining_weight.into(),
                                last_mining_weight_update: miner_ledger.last_mining_weight_update,
                                last_claim: miner_ledger.last_claim,
                            },
                        )
                    })
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)?)
    }
}
