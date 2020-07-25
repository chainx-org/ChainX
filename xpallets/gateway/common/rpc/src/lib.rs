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

use xpallet_gateway_common_rpc_runtime_api::trustees::bitcoin::{
    BtcTrusteeIntentionProps, BtcTrusteeSessionInfo,
};
use xpallet_gateway_common_rpc_runtime_api::{
    AssetId, Chain, GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, WithdrawalLimit,
    XGatewayCommonApi as XGatewayCommonRuntimeApi,
};
use xpallet_support::RpcBalance;

/// XGatewayCommon RPC methods.
#[rpc]
pub trait XGatewayCommonApi<BlockHash, AccountId, RpcBalance> {
    #[rpc(name = "xgatewaycommon_withdrawalLimit")]
    fn withdrawal_limit(
        &self,
        asset_id: AssetId,
        at: Option<BlockHash>,
    ) -> Result<WithdrawalLimit<RpcBalance>>;

    #[rpc(name = "xgatewaycommon_verifyWithdrawal")]
    fn verify_withdrawal(
        &self,
        asset_id: AssetId,
        value: u64,
        addr: String,
        memo: String,
        at: Option<BlockHash>,
    ) -> Result<()>;

    #[rpc(name = "xgatewaycommon_trusteeMultisigs")]
    fn multisigs(&self, at: Option<BlockHash>) -> Result<BTreeMap<Chain, AccountId>>;

    #[rpc(name = "xgatewaycommon_bitcoinTrusteeProperties")]
    fn btc_trustee_properties(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BtcTrusteeIntentionProps>;

    #[rpc(name = "xgatewaycommon_bitcoinTrusteeSessionInfo")]
    fn btc_trustee_session_info(
        &self,
        at: Option<BlockHash>,
    ) -> Result<BtcTrusteeSessionInfo<AccountId>>;

    #[rpc(name = "xgatewaycommon_bitcoinGenerateTrusteeSessionInfo")]
    fn btc_generate_trustee_session_info(
        &self,
        candidates: Vec<AccountId>,
        at: Option<BlockHash>,
    ) -> Result<BtcTrusteeSessionInfo<AccountId>>;
}

/// A struct that implements the [`XStakingApi`].
pub struct XGatewayCommon<C, B, AccountId, Balance> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
    _marker2: std::marker::PhantomData<AccountId>,
    _marker3: std::marker::PhantomData<Balance>,
}

impl<C, B, AccountId, Balance> XGatewayCommon<C, B, AccountId, Balance> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
            _marker2: Default::default(),
            _marker3: Default::default(),
        }
    }
}

impl<C, Block, AccountId, Balance> XGatewayCommon<C, Block, AccountId, Balance>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XGatewayCommonRuntimeApi<Block, AccountId, Balance>,
    AccountId: Codec + Send + Sync + 'static,
    Balance: Codec + Send + Sync + 'static,
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

impl<C, Block, AccountId, Balance>
    XGatewayCommonApi<<Block as BlockT>::Hash, AccountId, RpcBalance<Balance>>
    for XGatewayCommon<C, Block, AccountId, Balance>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XGatewayCommonRuntimeApi<Block, AccountId, Balance>,
    AccountId: Codec + Send + Sync + 'static,
    Balance: Codec + Send + Sync + 'static + From<u64>,
{
    fn withdrawal_limit(
        &self,
        asset_id: AssetId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<WithdrawalLimit<RpcBalance<Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let result = api
            .withdrawal_limit(&at, asset_id)
            .map_err(|e| runtime_error_into_rpc_err(e))?
            .map(|src| WithdrawalLimit {
                minimal_withdrawal: src.minimal_withdrawal.into(),
                fee: src.fee.into(),
            })
            .map_err(|e| runtime_error_into_rpc_err(e))?;
        Ok(result)
    }

    fn verify_withdrawal(
        &self,
        asset_id: AssetId,
        value: u64,
        addr: String,
        memo: String,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<()> {
        let value: Balance = Balance::from(value);
        let addr = if addr.starts_with("0x") {
            hex::decode(&addr[2..]).map_err(|err| RpcError {
                code: ErrorCode::ServerError(RUNTIME_ERROR + 10),
                message: "Decode to hex error".into(),
                data: Some(format!("{:?}", err).into()),
            })?
        } else {
            hex::decode(&addr).unwrap_or(addr.into_bytes())
        };
        let memo = memo.into_bytes();

        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
        api.verify_withdrawal(&at, asset_id, value, addr, memo.into())
            .map_err(|e| runtime_error_into_rpc_err(e))?
            .map_err(|e| runtime_error_into_rpc_err(e))?;
        Ok(())
    }

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
    ) -> Result<BtcTrusteeIntentionProps> {
        let props = self.generic_trustee_properties(Chain::Bitcoin, who, at)?;
        BtcTrusteeIntentionProps::try_from(props).map_err(|_| RpcError {
            code: ErrorCode::ServerError(RUNTIME_ERROR + 2),
            message: "Decode generic data error, should not happen".into(),
            data: None,
        })
    }

    fn btc_trustee_session_info(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BtcTrusteeSessionInfo<AccountId>> {
        let info = self.generic_trustee_session_info(Chain::Bitcoin, at)?;
        BtcTrusteeSessionInfo::<_>::try_from(info).map_err(|_| RpcError {
            code: ErrorCode::ServerError(RUNTIME_ERROR + 2),
            message: "Decode generic data error, should not happen".into(),
            data: None,
        })
    }

    fn btc_generate_trustee_session_info(
        &self,
        candidates: Vec<AccountId>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BtcTrusteeSessionInfo<AccountId>> {
        let info = self.generate_generic_trustee_session_info(Chain::Bitcoin, candidates, at)?;
        BtcTrusteeSessionInfo::<_>::try_from(info).map_err(|_| RpcError {
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
