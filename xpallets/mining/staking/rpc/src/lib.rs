//! RPC interface for the transaction payment module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use xpallet_mining_staking_rpc_runtime_api::XStakingApi as XStakingRuntimeApi;

/// XStaking RPC methods.
#[rpc]
pub trait XStakingApi<BlockHash, AccountId> {
    /// Executes a call to a contract.
    ///
    /// This call is performed locally without submitting any transactions. Thus executing this
    /// won't change any state. Nonetheless, the calling state-changing contracts is still possible.
    ///
    /// This method is useful for calling getter-like methods on contracts.
    #[rpc(name = "xstaking_getValidators")]
    fn validators(&self, at: Option<BlockHash>) -> Result<Vec<AccountId>>;
}

/// A struct that implements the [`XStakingApi`].
pub struct XStaking<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> XStaking<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId> XStakingApi<<Block as BlockT>::Hash, AccountId> for XStaking<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XStakingRuntimeApi<Block, AccountId>,
    AccountId: Codec,
{
    fn validators(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<AccountId>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
                // If the block hash is not supplied assume the best block.
                self.client.info().best_hash));

        let result = api
            .validators(&at)
            .map_err(|e| runtime_error_into_rpc_err(e))?;

        Ok(result)
    }
}

/// Error type of this RPC api.
pub enum Error {
    /// The transaction was not decodable.
    DecodeError,
    /// The call to runtime failed.
    RuntimeError,
}

impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
            Error::DecodeError => 2,
        }
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
