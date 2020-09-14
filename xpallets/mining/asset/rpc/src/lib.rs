// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! RPC interface for the transaction payment module.

use chainx_primitives::AssetId;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::collections::btree_map::BTreeMap;
use std::sync::Arc;
use xpallet_mining_asset::{MiningAssetInfo, RpcMinerLedger};
use xpallet_mining_asset_rpc_runtime_api::XMiningAssetApi as XMiningAssetRuntimeApi;
use xpallet_support::RpcBalance;

/// XMiningAsset RPC methods.
#[rpc]
pub trait XMiningAssetApi<BlockHash, AccountId, RpcBalance, BlockNumber> {
    /// Get overall information about all mining assets.
    #[rpc(name = "xminingasset_getMiningAssets")]
    fn mining_assets(
        &self,
        at: Option<BlockHash>,
    ) -> Result<Vec<MiningAssetInfo<AccountId, RpcBalance, BlockNumber>>>;

    /// Get the asset mining dividends info given the asset miner AccountId.
    #[rpc(name = "xminingasset_getDividendByAccount")]
    fn mining_dividend(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AssetId, RpcBalance>>;

    /// Get the mining ledger details given the asset miner AccountId.
    #[rpc(name = "xminingasset_getMinerLedgerByAccount")]
    fn miner_ledger(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AssetId, RpcMinerLedger<BlockNumber>>>;
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

impl<C, Block, AccountId, Balance, BlockNumber>
    XMiningAssetApi<<Block as BlockT>::Hash, AccountId, RpcBalance<Balance>, BlockNumber>
    for XMiningAsset<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XMiningAssetRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    AccountId: Codec,
    Balance: Codec,
    BlockNumber: Codec,
{
    fn mining_assets(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<MiningAssetInfo<AccountId, RpcBalance<Balance>, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api.mining_assets(&at).map_err(runtime_error_into_rpc_err)?)
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
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn miner_ledger(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, RpcMinerLedger<BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .miner_ledger(&at, who)
            .map_err(runtime_error_into_rpc_err)?)
    }
}

/// Error type of this RPC api.
pub enum Error {
    /// The transaction was not decodable.
    DecodeError,
    /// The call to runtime failed.
    RuntimeError,
}

impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
            Error::DecodeError => 2,
        }
    }
}

const RUNTIME_ERROR: i64 = 1;

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> RpcError {
    RpcError {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(format!("{:?}", err).into()),
    }
}
