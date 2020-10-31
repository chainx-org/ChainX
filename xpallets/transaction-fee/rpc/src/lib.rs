//! RPC interface for the transaction fee module.

use codec::{Codec, Decode};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use pallet_transaction_payment_rpc::Error;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};
use std::sync::Arc;
use xpallet_transaction_fee_rpc_runtime_api::RuntimeDispatchInfo;

pub use self::gen_client::Client as TransactionFeeClient;
pub use xpallet_transaction_fee_rpc_runtime_api::TransactionFeeApi as TransactionFeeRuntimeApi;

#[rpc]
pub trait TransactionFeeApi<BlockHash, ResponseType> {
    #[rpc(name = "payment_queryDetailedInfo")]
    fn query_detailed_info(&self, encoded_xt: Bytes, at: Option<BlockHash>)
        -> Result<ResponseType>;
}

/// A struct that implements the [`TransactionFeeApi`].
pub struct TransactionFee<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> TransactionFee<C, P> {
    /// Create new `TransactionPayment` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, Balance> TransactionFeeApi<<Block as BlockT>::Hash, RuntimeDispatchInfo<Balance>>
    for TransactionFee<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: TransactionFeeRuntimeApi<Block, Balance>,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
{
    fn query_detailed_info(
        &self,
        encoded_xt: Bytes,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<RuntimeDispatchInfo<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let encoded_len = encoded_xt.len() as u32;

        let uxt: Block::Extrinsic = Decode::decode(&mut &*encoded_xt).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::DecodeError.into()),
            message: "Unable to query dispatch info.".into(),
            data: Some(format!("{:?}", e).into()),
        })?;

        api.query_detailed_info(&at, uxt, encoded_len)
            .map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: "Unable to query dispatch info.".into(),
                data: Some(format!("{:?}", e).into()),
            })
    }
}
