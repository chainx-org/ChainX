// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! RPC interface for the transaction verification.
use jsonrpc_derive::rpc;
use std::sync::Arc;
use std::vec::Vec;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use xp_rpc::{runtime_error_into_rpc_err, Result};
use xpallet_gateway_bitcoin_rpc_runtime_api::XGatewayBitcoinApi as XGatewayBitcoinRuntimeApi;

pub struct XGatewayBitcoin<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> XGatewayBitcoin<C, B> {
    /// Create new `XGatewayBitcoin` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

#[rpc]
pub trait XGatewayBitcoinApi<BlockHash> {
    /// Verify transaction is valid
    #[rpc(name = "xgatewaybitcoin_verifyTxValid")]
    fn verify_tx_valid(
        &self,
        raw_tx: String,
        withdrawal_id_list: Vec<u32>,
        full_amount: bool,
        at: Option<BlockHash>,
    ) -> Result<bool>;
}

impl<C, Block> XGatewayBitcoinApi<<Block as BlockT>::Hash> for XGatewayBitcoin<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XGatewayBitcoinRuntimeApi<Block>,
{
    fn verify_tx_valid(
        &self,
        raw_tx: String,
        withdrawal_id_list: Vec<u32>,
        full_amount: bool,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<bool> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let raw_tx = hex::decode(raw_tx).map_err(runtime_error_into_rpc_err)?;
        let result = api
            .verify_tx_valid(&at, raw_tx, withdrawal_id_list, full_amount)
            .map_err(runtime_error_into_rpc_err)?
            .map_err(runtime_error_into_rpc_err)?;
        Ok(result)
    }
}
