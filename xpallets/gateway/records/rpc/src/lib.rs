// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use xp_rpc::{runtime_error_into_rpc_err, Result};

use xpallet_gateway_records_rpc_runtime_api::{
    AssetId, Chain, Withdrawal, WithdrawalRecordId, WithdrawalState,
    XGatewayRecordsApi as GatewayRecordsRuntimeApi,
};
use xpallet_support::try_addr;

pub struct XGatewayRecords<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> XGatewayRecords<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

#[rpc]
pub trait XGatewayRecordsApi<BlockHash, AccountId, Balance, BlockNumber>
where
    Balance: Display + FromStr,
{
    /// Return current withdraw list(include Applying and Processing withdraw state)
    #[rpc(name = "xgatewayrecords_withdrawalList")]
    fn withdrawal_list(
        &self,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<WithdrawalRecordId, RpcWithdrawalRecord<AccountId, Balance, BlockNumber>>>;

    /// Return current withdraw list for a chain(include Applying and Processing withdraw state)
    #[rpc(name = "xgatewayrecords_withdrawalListByChain")]
    fn withdrawal_list_by_chain(
        &self,
        chain: Chain,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<WithdrawalRecordId, RpcWithdrawalRecord<AccountId, Balance, BlockNumber>>>;

    /// Return current pending withdraw list for a chain
    #[rpc(name = "xgatewayrecords_pendingWithdrawalList")]
    fn pending_withdrawal_list(
        &self,
        chain: Chain,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<WithdrawalRecordId, RpcWithdrawalRecord<AccountId, Balance, BlockNumber>>>;
}

impl<C, Block, AccountId, Balance, BlockNumber>
    XGatewayRecordsApi<<Block as BlockT>::Hash, AccountId, Balance, BlockNumber>
    for XGatewayRecords<C, Block>
where
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: GatewayRecordsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    Block: BlockT,
    AccountId: Clone + Display + FromStr + Codec,
    Balance: Clone + Display + FromStr + Codec,
    BlockNumber: Clone + Display + Codec,
{
    fn withdrawal_list(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<u32, RpcWithdrawalRecord<AccountId, Balance, BlockNumber>>> {
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
    ) -> Result<BTreeMap<u32, RpcWithdrawalRecord<AccountId, Balance, BlockNumber>>> {
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
    ) -> Result<BTreeMap<u32, RpcWithdrawalRecord<AccountId, Balance, BlockNumber>>> {
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
pub struct RpcWithdrawalRecord<AccountId, Balance: Display + FromStr, BlockNumber> {
    pub asset_id: AssetId,
    pub applicant: AccountId,
    #[serde(with = "xp_rpc::serde_num_str")]
    pub balance: Balance,
    pub addr: String,
    pub ext: String,
    pub height: BlockNumber,
    pub state: WithdrawalState,
}

impl<AccountId, Balance: Display + FromStr, BlockNumber>
    From<Withdrawal<AccountId, Balance, BlockNumber>>
    for RpcWithdrawalRecord<AccountId, Balance, BlockNumber>
{
    fn from(record: Withdrawal<AccountId, Balance, BlockNumber>) -> Self {
        Self {
            asset_id: record.asset_id,
            applicant: record.applicant,
            balance: record.balance,
            addr: format!("{:?}", try_addr!(record.addr)),
            ext: String::from_utf8_lossy(record.ext.as_ref()).into_owned(),
            height: record.height,
            state: record.state,
        }
    }
}
