// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Zero},
};

use xp_rpc::{runtime_error_into_rpc_err, Result, RpcBalance};

use xpallet_btc_ledger_runtime_api::{
    BtcLedgerApi as BtcLedgerRuntimeApi,
};

pub struct BtcLedger<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> BtcLedger<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

#[rpc]
pub trait BtcLedgerApi<BlockHash, AccountId, Balance>
where
    Balance: Display + FromStr,
{
    /// Return balance for an account
    #[rpc(name = "btc_getBalance")]
    fn btc_balance(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<RpcBalance<Balance>>;

    /// Return total balance of BTC
    #[rpc(name = "btc_getTotal")]
    fn btc_total(
        &self,
        at: Option<BlockHash>,
    ) -> Result<RpcBalance<Balance>>;
}

impl<C, Block, AccountId, Balance> BtcLedgerApi<<Block as BlockT>::Hash, AccountId, Balance>
    for BtcLedger<C, Block>
where
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: BtcLedgerRuntimeApi<Block, AccountId, Balance>,
    Block: BlockT,
    AccountId: Clone + Display + Codec,
    Balance: Clone + Copy + Display + FromStr + Codec + Zero,
{
    fn btc_balance(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<RpcBalance<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.get_balance(&at, who).map(|b| b.into()).map_err(runtime_error_into_rpc_err)
    }

    fn btc_total(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<RpcBalance<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.get_total(&at).map(|b| b.into()).map_err(runtime_error_into_rpc_err)
    }
}
