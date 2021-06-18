// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! RPC interface for the transaction fee module.

use codec::{Codec, Decode};
use jsonrpc_core::serde_json::Number;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_runtime::generic::Header;
use std::fmt::Debug;
use std::sync::Arc;

use chainx_runtime::impls::ChargeExtraFee;
use chainx_runtime::UncheckedExtrinsic;
use pallet_transaction_payment::InclusionFee;
use pallet_transaction_payment_rpc::Error;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{
    generic,
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};
use xp_rpc::{RpcBalance, RpcU128};
use xpallet_transaction_fee::FeeDetails;
//pub use xpallet_transaction_fee_rpc_runtime_api::XTransactionFeeApi as XTransactionFeeRuntimeApi;
pub use pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi as XTransactionFeeRuntimeApi;
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
    RpcU128<Balance>: From<u32> + From<u128>,
{
    fn query_fee_details(
        &self,
        encoded_xt: Bytes,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<FeeDetails<RpcBalance<Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let encoded_len = encoded_xt.len() as u32;

        let uxt: <Block as BlockT>::Extrinsic =
            Decode::decode(&mut &*encoded_xt).map_err(into_rpc_err)?;

        let result = api
            .query_fee_details(&at, uxt, encoded_len)
            .map(|fee_details| pallet_transaction_payment::FeeDetails {
                inclusion_fee: fee_details.inclusion_fee.map(|fee| InclusionFee {
                    base_fee: fee.base_fee.into(),
                    len_fee: fee.len_fee.into(),
                    adjusted_weight_fee: fee.adjusted_weight_fee.into(),
                }),
                tip: fee_details.tip.into(),
            })
            .map_err(into_rpc_err);
        let base = match result {
            Ok(res) => res,
            Err(Error) => return Err(Error),
        };
        let uxt_clone = uxt.clone();
        if let Some(extra_fee) = ChargeExtraFee::has_extra_fee(&uxt.function) {
            let base_clone = base.clone();
            let total = match base.inclusion_fee {
                Some(fee) => fee
                    .base_fee
                    .saturating_add(fee.len_fee)
                    .saturating_add(fee.adjusted_weight_fee)
                    .saturating_add(base.tip),
                None => 0,
            };
            Ok(FeeDetails {
                inclusion_fee: base_clone.inclusion_fee,
                tip: base.tip,
                extra_fee: extra_fee.into(),
                final_fee: total + extra_fee,
            })
        } else {
            Ok(FeeDetails {
                inclusion_fee: base.inclusion_fee,
                tip: base.tip,
                extra_fee: 0u32.into(),
                final_fee: base.tip,
            })
        }
    }
}

fn into_rpc_err(err: impl Debug) -> RpcError {
    RpcError {
        code: ErrorCode::ServerError(Error::RuntimeError.into()),
        message: "Unable to query dispatch info.".into(),
        data: Some(format!("{:?}", err).into()),
    }
}
