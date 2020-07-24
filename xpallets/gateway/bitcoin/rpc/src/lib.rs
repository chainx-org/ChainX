//! RPC interface for the transaction payment module.

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use xpallet_gateway_bitcoin_rpc_runtime_api::{
    XGatewayBitcoinApi as XGatewayBitcoinRuntimeApi, BTCTrusteeSessionInfo
};

/// XGatewayCommon RPC methods.
#[rpc]
pub trait XGatewayBitcoinApi<BlockHash, AccountId> {
    #[rpc(name = "xgatewaybitcoin_generateTrusteeInfo")]
    fn mock_trustee_info(&self, candidates: Vec<AccountId>, at: Option<BlockHash>) -> Result<BTCTrusteeSessionInfo<AccountId>>;
}

/// A struct that implements the [`XStakingApi`].
pub struct XGatewayBitcoin<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B, AccountId> XGatewayBitcoin<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}


impl<C, Block, AccountId> XGatewayBitcoinApi<<Block as BlockT>::Hash, AccountId>
    for XGatewayBitcoin<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XGatewayBitcoinRuntimeApi<Block, AccountId>,
    AccountId: Codec + Send + Sync + 'static,
{
    fn mock_trustee_info(&self, candidates: Vec<AccountId>, at: Option<<Block as BlockT>::Hash>) -> Result<BTCTrusteeSessionInfo<AccountId>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.generate_trustee_info(&at, candidates)
            .map(|map| {
                map.into_iter()
                    .map(|(id, withdrawal)| (id, withdrawal.into()))
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)
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
