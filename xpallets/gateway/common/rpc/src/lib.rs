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

use xpallet_gateway_common_rpc_runtime_api::{
    trustees, Chain, GenericTrusteeIntentionProps, GenericTrusteeSessionInfo,
    XGatewayCommonApi as XGatewayCommonRuntimeApi,
};

/// XGatewayCommon RPC methods.
#[rpc]
pub trait XGatewayCommonApi<BlockHash, AccountId> {
    #[rpc(name = "xgatewaycommon_trusteeMultisigs")]
    fn multisigs(&self, at: Option<BlockHash>) -> Result<BTreeMap<Chain, AccountId>>;

    #[rpc(name = "xgatewaycommon_bitcoinTrusteeProperties")]
    fn btc_trustee_properties(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<trustees::bitcoin::BTCTrusteeIntentionProps>;

    #[rpc(name = "xgatewaycommon_bitcoinTrusteeSessionInfo")]
    fn btc_trustee_session_info(
        &self,
        at: Option<BlockHash>,
    ) -> Result<trustees::bitcoin::BTCTrusteeSessionInfo<AccountId>>;
}

/// A struct that implements the [`XStakingApi`].
pub struct XGatewayCommon<C, B, AccountId> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
    _marker2: std::marker::PhantomData<AccountId>,
}

impl<C, B, AccountId> XGatewayCommon<C, B, AccountId> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
            _marker2: Default::default(),
        }
    }
}

impl<C, Block, AccountId> XGatewayCommon<C, Block, AccountId>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XGatewayCommonRuntimeApi<Block, AccountId>,
    AccountId: Codec + Send + Sync + 'static,
{
    fn generic_trustee_properties(
        &self,
        chain: Chain,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<GenericTrusteeIntentionProps> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let result = api
            .trustee_properties(&at, chain, who)
            .map_err(|e| runtime_error_into_rpc_err(e))?;
        let result = result.ok_or(RpcError {
            code: ErrorCode::ServerError(RUNTIME_ERROR + 1),
            message: "Not exist".into(),
            data: None,
        })?;

        Ok(result)
    }

    fn generic_trustee_session_info(
        &self,
        chain: Chain,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<GenericTrusteeSessionInfo<AccountId>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let result = api
            .trustee_session_info(&at, chain)
            .map_err(|e| runtime_error_into_rpc_err(e))?;
        let result = result.ok_or(RpcError {
            code: ErrorCode::ServerError(RUNTIME_ERROR + 1),
            message: "Not exist".into(),
            data: None,
        })?;

        Ok(result)
    }

    fn generate_generic_trustee_session_info(
        &self,
        chain: Chain,
        candidates: Vec<AccountId>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<GenericTrusteeSessionInfo<AccountId>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let result = api
            .generate_trustee_session_info(&at, chain, candidates)
            .map_err(|e| runtime_error_into_rpc_err(e))?;
        let result = result.map_err(runtime_error_into_rpc_err)?;

        Ok(result)
    }
}

impl<C, Block, AccountId> XGatewayCommonApi<<Block as BlockT>::Hash, AccountId>
    for XGatewayCommon<C, Block, AccountId>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XGatewayCommonRuntimeApi<Block, AccountId>,
    AccountId: Codec + Send + Sync + 'static,
{
    fn multisigs(&self, at: Option<<Block as BlockT>::Hash>) -> Result<BTreeMap<Chain, AccountId>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
                // If the block hash is not supplied assume the best block.
                self.client.info().best_hash));

        let result = api
            .trustee_multisigs(&at)
            .map_err(|e| runtime_error_into_rpc_err(e))?;

        Ok(result)
    }

    fn btc_trustee_properties(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<trustees::bitcoin::BTCTrusteeIntentionProps> {
        let props = self.generic_trustee_properties(Chain::Bitcoin, who, at)?;
        trustees::bitcoin::BTCTrusteeIntentionProps::try_from(props).map_err(|_| RpcError {
            code: ErrorCode::ServerError(RUNTIME_ERROR + 2),
            message: "Decode generic data error, should not happen".into(),
            data: None,
        })
    }

    fn btc_trustee_session_info(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<trustees::bitcoin::BTCTrusteeSessionInfo<AccountId>> {
        let info = self.generic_trustee_session_info(Chain::Bitcoin, at)?;
        trustees::bitcoin::BTCTrusteeSessionInfo::<_>::try_from(info).map_err(|_| RpcError {
            code: ErrorCode::ServerError(RUNTIME_ERROR + 2),
            message: "Decode generic data error, should not happen".into(),
            data: None,
        })
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
