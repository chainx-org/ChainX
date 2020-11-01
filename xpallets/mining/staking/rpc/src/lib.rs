// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! RPC interface for the transaction payment module.

use std::collections::btree_map::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use xp_rpc::runtime_error_into_rpc_err;

use xpallet_support::{RpcBalance, RpcVoteWeight};

use xpallet_mining_staking_rpc_runtime_api::{
    NominatorInfo, NominatorLedger, Unbonded, ValidatorInfo, ValidatorLedger,
    XStakingApi as XStakingRuntimeApi,
};

/// XStaking RPC methods.
#[rpc]
pub trait XStakingApi<BlockHash, AccountId, Balance, VoteWeight, BlockNumber>
where
    AccountId: Ord,
    Balance: Display + FromStr,
    VoteWeight: Display + FromStr,
{
    /// Get overall information about all potential validators
    #[rpc(name = "xstaking_getValidators")]
    fn validators(
        &self,
        at: Option<BlockHash>,
    ) -> Result<
        Vec<ValidatorInfo<AccountId, RpcBalance<Balance>, RpcVoteWeight<VoteWeight>, BlockNumber>>,
    >;

    /// Get overall information given the validator AccountId.
    #[rpc(name = "xstaking_getValidatorByAccount")]
    fn validator_info_of(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<ValidatorInfo<AccountId, RpcBalance<Balance>, RpcVoteWeight<VoteWeight>, BlockNumber>>;

    /// Get the staking dividends info given the staker AccountId.
    #[rpc(name = "xstaking_getDividendByAccount")]
    fn staking_dividend_of(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AccountId, RpcBalance<Balance>>>;

    /// Get the nomination details given the staker AccountId.
    #[rpc(name = "xstaking_getNominationByAccount")]
    fn nomination_details_of(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<
        BTreeMap<
            AccountId,
            NominatorLedger<RpcBalance<Balance>, RpcVoteWeight<VoteWeight>, BlockNumber>,
        >,
    >;

    /// Get individual nominator information given the nominator AccountId.
    #[rpc(name = "xstaking_getNominatorByAccount")]
    fn nominator_info_of(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<NominatorInfo<BlockNumber>>;
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

impl<C, Block, AccountId, Balance, VoteWeight, BlockNumber>
    XStakingApi<<Block as BlockT>::Hash, AccountId, Balance, VoteWeight, BlockNumber>
    for XStaking<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XStakingRuntimeApi<Block, AccountId, Balance, VoteWeight, BlockNumber>,
    AccountId: Codec + Ord,
    Balance: Codec + Display + FromStr,
    VoteWeight: Codec + Display + FromStr,
    BlockNumber: Codec,
{
    fn validators(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<
        Vec<ValidatorInfo<AccountId, RpcBalance<Balance>, RpcVoteWeight<VoteWeight>, BlockNumber>>,
    > {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .validators(&at)
            .map(|validators| {
                validators
                    .into_iter()
                    .map(|validator| ValidatorInfo {
                        account: validator.account,
                        profile: validator.profile,
                        ledger: ValidatorLedger {
                            total_nomination: validator.ledger.total_nomination.into(),
                            last_total_vote_weight: validator.ledger.last_total_vote_weight.into(),
                            last_total_vote_weight_update: validator
                                .ledger
                                .last_total_vote_weight_update,
                        },
                        is_validating: validator.is_validating,
                        self_bonded: validator.self_bonded.into(),
                        reward_pot_account: validator.reward_pot_account,
                        reward_pot_balance: validator.reward_pot_balance.into(),
                    })
                    .collect::<Vec<_>>()
            })
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn validator_info_of(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<ValidatorInfo<AccountId, RpcBalance<Balance>, RpcVoteWeight<VoteWeight>, BlockNumber>>
    {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .validator_info_of(&at, who)
            .map(|validator| ValidatorInfo {
                account: validator.account,
                profile: validator.profile,
                ledger: ValidatorLedger {
                    total_nomination: validator.ledger.total_nomination.into(),
                    last_total_vote_weight: validator.ledger.last_total_vote_weight.into(),
                    last_total_vote_weight_update: validator.ledger.last_total_vote_weight_update,
                },
                is_validating: validator.is_validating,
                self_bonded: validator.self_bonded.into(),
                reward_pot_account: validator.reward_pot_account,
                reward_pot_balance: validator.reward_pot_balance.into(),
            })
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
            .map(|staking_dividend| {
                staking_dividend
                    .into_iter()
                    .map(|(account, balance)| (account, balance.into()))
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn nomination_details_of(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<
        BTreeMap<
            AccountId,
            NominatorLedger<RpcBalance<Balance>, RpcVoteWeight<VoteWeight>, BlockNumber>,
        >,
    > {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .nomination_details_of(&at, who)
            .map(|nomination_details| {
                nomination_details
                    .into_iter()
                    .map(|(account, nominator_ledger)| {
                        (
                            account,
                            NominatorLedger {
                                nomination: nominator_ledger.nomination.into(),
                                last_vote_weight: nominator_ledger.last_vote_weight.into(),
                                last_vote_weight_update: nominator_ledger.last_vote_weight_update,
                                unbonded_chunks: nominator_ledger
                                    .unbonded_chunks
                                    .into_iter()
                                    .map(|unbonded| Unbonded {
                                        value: unbonded.value.into(),
                                        locked_until: unbonded.locked_until,
                                    })
                                    .collect(),
                            },
                        )
                    })
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)?)
    }

    fn nominator_info_of(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<NominatorInfo<BlockNumber>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .nominator_info_of(&at, who)
            .map_err(runtime_error_into_rpc_err)?)
    }
}
