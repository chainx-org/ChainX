//! RPC interface for the transaction payment module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::collections::btree_map::BTreeMap;
use std::sync::Arc;
use xpallet_mining_staking::{RpcNominatorLedger, ValidatorInfo};
use xpallet_mining_staking_rpc_runtime_api::XStakingApi as XStakingRuntimeApi;
use xpallet_support::RpcBalance;

/// XStaking RPC methods.
#[rpc]
pub trait XStakingApi<BlockHash, AccountId, RpcBalance, BlockNumber>
where
    AccountId: Ord,
{
    /// Get overall information about all potential validators
    #[rpc(name = "xstaking_getValidators")]
    fn validators(
        &self,
        at: Option<BlockHash>,
    ) -> Result<Vec<ValidatorInfo<AccountId, RpcBalance, BlockNumber>>>;

    /// Get overall information given the validator AccountId.
    #[rpc(name = "xstaking_getValidatorByAccount")]
    fn validator_info_of(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<ValidatorInfo<AccountId, RpcBalance, BlockNumber>>;

    /// Get the staking dividends info given the staker AccountId.
    #[rpc(name = "xstaking_getDividendByAccount")]
    fn staking_dividend_of(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AccountId, RpcBalance>>;

    /// Get the nomination details given the staker AccountId.
    #[rpc(name = "xstaking_getNominationByAccount")]
    fn nomination_details_of(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AccountId, RpcNominatorLedger<RpcBalance, BlockNumber>>>;
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

impl<C, Block, AccountId, Balance, BlockNumber>
    XStakingApi<<Block as BlockT>::Hash, AccountId, RpcBalance<Balance>, BlockNumber>
    for XStaking<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XStakingRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    AccountId: Codec + Ord,
    Balance: Codec,
    BlockNumber: Codec,
{
    fn validators(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<ValidatorInfo<AccountId, RpcBalance<Balance>, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api.validators(&at).map_err(runtime_error_into_rpc_err)?)
    }

    fn validator_info_of(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<ValidatorInfo<AccountId, RpcBalance<Balance>, BlockNumber>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .validator_info_of(&at, who)
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn staking_dividend_of(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AccountId, RpcBalance<Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .staking_dividend_of(&at, who)
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn nomination_details_of(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AccountId, RpcNominatorLedger<RpcBalance<Balance>, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .nomination_details_of(&at, who)
            .map_err(runtime_error_into_rpc_err)?)
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
