//! RPC interface for the DEX Spot module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use xpallet_dex_spot::{Depth, FullPairInfo, RpcOrder, TradingPairId};
use xpallet_dex_spot_rpc_runtime_api::XSpotApi as XSpotRuntimeApi;
use xpallet_support::{RpcBalance, RpcPrice};

/// XSpot RPC methods.
#[rpc]
pub trait XSpotApi<BlockHash, AccountId, RpcBalance, BlockNumber, RpcPrice> {
    /// Get the overall info of all trading pairs.
    #[rpc(name = "xspot_getTradingPairs")]
    fn trading_pairs(
        &self,
        at: Option<BlockHash>,
    ) -> Result<Vec<FullPairInfo<RpcPrice, BlockNumber>>>;

    /// Get the orders of an account.
    #[rpc(name = "xspot_getOrdersByAccount")]
    fn orders(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<Vec<RpcOrder<TradingPairId, AccountId, RpcBalance, RpcPrice, BlockNumber>>>;

    /// Get the depth of a trading pair.
    #[rpc(name = "xspot_getDepth")]
    fn depth(
        &self,
        pair_id: TradingPairId,
        depth_size: u32,
        at: Option<BlockHash>,
    ) -> Result<Option<Depth<RpcPrice, RpcBalance>>>;
}

/// A struct that implements the [`XSpotApi`].
pub struct XSpot<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> XSpot<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId, Balance, BlockNumber, Price>
    XSpotApi<<Block as BlockT>::Hash, AccountId, RpcBalance<Balance>, BlockNumber, RpcPrice<Price>>
    for XSpot<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XSpotRuntimeApi<Block, AccountId, Balance, BlockNumber, Price>,
    AccountId: Codec,
    Balance: Codec,
    BlockNumber: Codec,
    Price: Codec,
{
    fn trading_pairs(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<FullPairInfo<RpcPrice<Price>, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api.trading_pairs(&at).map_err(runtime_error_into_rpc_err)?)
    }

    fn orders(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<
        Vec<RpcOrder<TradingPairId, AccountId, RpcBalance<Balance>, RpcPrice<Price>, BlockNumber>>,
    > {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api.orders(&at, who).map_err(runtime_error_into_rpc_err)?)
    }

    fn depth(
        &self,
        pair_id: TradingPairId,
        depth_size: u32,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Depth<RpcPrice<Price>, RpcBalance<Balance>>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .depth(&at, pair_id, depth_size)
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
