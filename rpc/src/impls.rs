use std::convert::TryInto;
use std::sync::Arc;

use codec::Decode;
use jsonrpc_core::{Error, ErrorCode, Result};
use serde_json::{json, Value};

use sc_client_api::{backend::Backend, CallExecutor, StorageProvider};
use sc_service::client::Client;
use sp_api::{BlockT, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::generic::BlockId;
use sp_state_machine::Backend as Backend2;

use frame_support::StorageMap;

use chainx_runtime::{opaque::Block, AccountId, Runtime};

use crate::apis::CiApi;
use crate::errors::XRpcErr;

pub struct CiRpc<BE, E, RA> {
    client: Arc<Client<BE, E, Block, RA>>,
}

impl<BE, E, RA> CiRpc<BE, E, RA>
where
    BE: Backend<Block>,
    BE::State: sp_state_machine::backend::Backend<sp_runtime::traits::BlakeTwo256>,
    E: CallExecutor<Block> + Clone + Send + Sync,
    RA: Send + Sync + 'static,
    Client<BE, E, Block, RA>: Send
        + Sync
        + 'static
        + ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + StorageProvider<Block, BE>,
{
    /// Create new `CiRpc` with the given reference to the client.
    pub fn new(client: Arc<Client<BE, E, Block, RA>>) -> Self {
        CiRpc { client }
    }
    /// Returns given block hash or best block hash if None is passed.
    fn block_or_best(&self, hash: Option<<Block as BlockT>::Hash>) -> <Block as BlockT>::Hash {
        hash.unwrap_or_else(|| self.client.info().best_hash)
    }

    fn state(&self, hash: Option<<Block as BlockT>::Hash>) -> Result<BE::State> {
        let b = BlockId::Hash(self.block_or_best(hash));
        self.client.state_at(&b).map_err(|e| Error {
            code: ErrorCode::InternalError,
            message: format!("get state for block:{:?} error:{:?}", b, e),
            data: None,
        })
    }

    fn pickout<ReturnValue: Decode>(state: &BE::State, key: &[u8]) -> Result<Option<ReturnValue>> {
        let d = state.storage(&key).map_err(|e| Error {
            code: ErrorCode::InternalError,
            message: format!("get storage for key:0x{:} error:{:?}", hex::encode(key), e),
            data: None,
        })?;
        match d {
            None => Ok(None),
            Some(value) => Decode::decode(&mut value.as_slice())
                .map(Some)
                .map_err(|e| Error {
                    code: ErrorCode::InternalError,
                    message: format!(
                        "decode storage value:0x{:?} error:{:?}",
                        value.as_slice(),
                        e
                    ),
                    data: None,
                }),
        }
    }
}

impl<BE, E, RA> CiApi<<Block as BlockT>::Hash> for CiRpc<BE, E, RA>
where
    BE: Backend<Block>,
    BE::State: sp_state_machine::backend::Backend<sp_runtime::traits::BlakeTwo256>,
    E: CallExecutor<Block> + Clone + Send + Sync,
    RA: Send + Sync + 'static,
    Client<BE, E, Block, RA>: Send
        + Sync
        + 'static
        + ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + StorageProvider<Block, BE>,
{
}
