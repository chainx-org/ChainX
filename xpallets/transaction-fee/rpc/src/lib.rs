// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! RPC interface for the transaction fee module.

use std::fmt::Debug;
use std::sync::Arc;

use codec::{Codec, Decode};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};

use pallet_transaction_payment_rpc::Error;

use xpallet_support::RpcBalance;
use xpallet_transaction_fee_rpc_runtime_api::{FeeDetails, InclusionFee};

pub use xpallet_transaction_fee_rpc_runtime_api::XTransactionFeeApi as XTransactionFeeRuntimeApi;

#[rpc]
pub trait XTransactionFeeApi<BlockHash, ResponseType> {
    #[rpc(name = "xfee_queryDetails")]
    fn query_fee_details(&self, encoded_xt: Bytes, at: Option<BlockHash>) -> Result<ResponseType>;
}

/// A struct that implements the [`TransactionFeeApi`].
pub struct XTransactionFee<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> XTransactionFee<C, P> {
    /// Create new `TransactionPayment` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, Balance> XTransactionFeeApi<<Block as BlockT>::Hash, FeeDetails<RpcBalance<Balance>>>
    for XTransactionFee<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XTransactionFeeRuntimeApi<Block, Balance>,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
{
    fn query_fee_details(
        &self,
        encoded_xt: Bytes,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<FeeDetails<RpcBalance<Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let encoded_len = encoded_xt.len() as u32;

        let uxt: Block::Extrinsic = Decode::decode(&mut &*encoded_xt).map_err(into_rpc_err)?;

        api.query_fee_details(&at, uxt, encoded_len)
            .map(|fee_details| FeeDetails {
                inclusion_fee: fee_details.inclusion_fee.map(|fee| InclusionFee {
                    base_fee: fee.base_fee.into(),
                    len_fee: fee.len_fee.into(),
                    adjusted_weight_fee: fee.adjusted_weight_fee.into(),
                }),
                tip: fee_details.tip.into(),
                final_fee: fee_details.final_fee.into(),
            })
            .map_err(into_rpc_err)
    }
}

fn into_rpc_err(err: impl Debug) -> RpcError {
    RpcError {
        code: ErrorCode::ServerError(Error::RuntimeError.into()),
        message: "Unable to query dispatch info.".into(),
        data: Some(format!("{:?}", err).into()),
    }
}
