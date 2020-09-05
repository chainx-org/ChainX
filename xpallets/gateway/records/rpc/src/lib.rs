use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use chainx_primitives::AssetId;
use xpallet_assets::Chain;
use xpallet_gateway_records_rpc_runtime_api::{
    Withdrawal, WithdrawalState, XGatewayRecordsApi as GatewayRecordsRuntimeApi,
};
use xpallet_support::{try_addr, RpcBalance};

pub struct XGatewayRecords<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> XGatewayRecords<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        XGatewayRecords {
            client,
            _marker: Default::default(),
        }
    }
}

#[rpc]
pub trait XGatewayRecordsApi<BlockHash, AccountId, Balance, BlockNumber>
where
    Balance: ToString + FromStr,
{
    /// Return current withdraw list(include Applying and Processing withdraw state)
    #[rpc(name = "xgatewayrecords_withdrawalList")]
    fn withdrawal_list(
        &self,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<u32, WithdrawalRecordForRpc<AccountId, Balance, BlockNumber>>>;

    /// Return current withdraw list for a chain(include Applying and Processing withdraw state)
    #[rpc(name = "xgatewayrecords_withdrawalListByChain")]
    fn withdrawal_list_by_chain(
        &self,
        chain: Chain,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<u32, WithdrawalRecordForRpc<AccountId, Balance, BlockNumber>>>;

    /// Return current pending withdraw list for a chain
    #[rpc(name = "xgatewayrecords_pendingWithdrawalList")]
    fn pending_withdrawal_list(
        &self,
        chain: Chain,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<u32, WithdrawalRecordForRpc<AccountId, Balance, BlockNumber>>>;
}

impl<C, Block, AccountId, Balance, BlockNumber>
    XGatewayRecordsApi<<Block as BlockT>::Hash, AccountId, Balance, BlockNumber>
    for XGatewayRecords<C, Block>
where
    C: sp_api::ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block>,
    C: Send + Sync + 'static,
    C::Api: GatewayRecordsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    Block: BlockT,
    AccountId: Clone + std::fmt::Display + Codec,
    Balance: Clone + std::fmt::Display + Codec + ToString + FromStr,
    BlockNumber: Clone + std::fmt::Display + Codec,
{
    fn withdrawal_list(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<u32, WithdrawalRecordForRpc<AccountId, Balance, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.withdrawal_list(&at)
            .map(|map| {
                map.into_iter()
                    .map(|(id, withdrawal)| (id, withdrawal.into()))
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)
    }

    fn withdrawal_list_by_chain(
        &self,
        chain: Chain,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<u32, WithdrawalRecordForRpc<AccountId, Balance, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.withdrawal_list_by_chain(&at, chain)
            .map(|map| {
                map.into_iter()
                    .map(|(id, withdrawal)| (id, withdrawal.into()))
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)
    }
    fn pending_withdrawal_list(
        &self,
        chain: Chain,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<u32, WithdrawalRecordForRpc<AccountId, Balance, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.withdrawal_list_by_chain(&at, chain)
            .map(|map| {
                map.into_iter()
                    .filter_map(|(id, withdrawal)| {
                        if withdrawal.state == WithdrawalState::Applying {
                            Some((id, withdrawal.into()))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalRecordForRpc<AccountId, Balance: ToString + FromStr, BlockNumber> {
    pub asset_id: AssetId,
    pub applicant: AccountId,
    pub balance: RpcBalance<Balance>,
    pub addr: String,
    pub ext: String,
    pub height: BlockNumber,
    pub state: WithdrawalState,
}

impl<AccountId, Balance: ToString + FromStr, BlockNumber>
    From<Withdrawal<AccountId, Balance, BlockNumber>>
    for WithdrawalRecordForRpc<AccountId, Balance, BlockNumber>
{
    fn from(record: Withdrawal<AccountId, Balance, BlockNumber>) -> Self {
        WithdrawalRecordForRpc {
            asset_id: record.asset_id,
            applicant: record.applicant,
            balance: record.balance.into(),
            addr: format!("{:?}", try_addr!(record.addr)),
            ext: format!("{:}", String::from_utf8_lossy(record.ext.as_ref())),
            height: record.height,
            state: record.state,
        }
    }
}

const RUNTIME_ERROR: i64 = 1;
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(format!("{:?}", err).into()),
    }
}
