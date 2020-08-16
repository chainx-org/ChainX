// Copyright 2018-2019 Chainpool.

#[macro_use]
mod cache;
#[macro_use]
mod utils;
mod chainx_impl;
mod chainx_trait;
mod error;
mod types;

use std::collections::btree_map::BTreeMap;
use std::result;
use std::sync::Arc;

use jsonrpc_derive::rpc;
use parity_codec::Decode;
use serde_json::Value;

use client::runtime_api::Metadata;
use primitives::crypto::UncheckedInto;
use primitives::storage::{StorageData, StorageKey};
use primitives::{Blake2Hasher, Bytes, H256};
use runtime_primitives::generic::BlockId;
use runtime_primitives::traits::Block as BlockT;
use runtime_primitives::traits::{Header as HeaderT, NumberFor, ProvideRuntimeApi, Zero};
use state_machine::Backend;

use support::storage::{StorageMap, StorageValue};

use chainx_primitives::{AccountId, AccountIdForRpc, AuthorityId, Balance, BlockNumber, Timestamp};
use chainx_runtime::Runtime;
use xr_primitives::{ContractExecResult, XRC20Selector};

use runtime_api::{
    xassets_api::XAssetsApi, xbridge_api::XBridgeApi, xcontracts_api::XContractsApi,
    xfee_api::XFeeApi, xmining_api::XMiningApi, xspot_api::XSpotApi, xstaking_api::XStakingApi,
};

use xassets::{Asset, AssetType, Chain, ChainT, Token};
use xbridge_common::types::{GenericAllSessionInfo, GenericTrusteeIntentionProps};
use xprocess::WithdrawalLimit;
use xspot::TradingPairIndex;
use xtokens::*;

pub use self::cache::set_cache_flag;
pub use self::chainx_trait::ChainXApi;
use self::error::{Error, Result};
pub use self::types::*;

/// Wrap runtime apis in ChainX API.
macro_rules! wrap_runtime_apis {
    (
        $(
            fn $fn_name:ident( $($arg:ident : $arg_ty:ty),* ) -> $ret:ty;
        )+
    ) => {
        $(
            #[allow(dead_code)]
            fn $fn_name(&self, number: BlockId<Block>, $($arg: $arg_ty),* ) -> result::Result<$ret, error::Error> {
                self.client.runtime_api().$fn_name( &number, $($arg),* ).map_err(Into::into)
            }
        )+
    };
}

/// ChainX API
pub struct ChainX<B, E, Block, RA>
where
    B: client::backend::Backend<Block, Blake2Hasher>,
    E: client::CallExecutor<Block, Blake2Hasher> + Clone + Send + Sync,
    Block: BlockT<Hash = H256>,
{
    client: Arc<client::Client<B, E, Block, RA>>,
}

impl<B, E, Block: BlockT, RA> ChainX<B, E, Block, RA>
where
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Clone + Send + Sync,
    Block: BlockT<Hash = H256> + 'static,
    RA: Send + Sync + 'static,
    client::Client<B, E, Block, RA>: ProvideRuntimeApi,
    <client::Client<B, E, Block, RA> as ProvideRuntimeApi>::Api: Metadata<Block>
        + XAssetsApi<Block>
        + XMiningApi<Block>
        + XSpotApi<Block>
        + XFeeApi<Block>
        + XStakingApi<Block>
        + XBridgeApi<Block>
        + XContractsApi<Block>,
{
    /// Create new ChainX API RPC handler.
    pub fn new(client: Arc<client::Client<B, E, Block, RA>>) -> Self {
        Self { client }
    }

    /// Generate storage key.
    fn storage_key(key: &[u8], hasher: Hasher) -> StorageKey {
        let hashed = match hasher {
            Hasher::TWOX128 => primitives::twox_128(key).to_vec(),
            Hasher::BLAKE2256 => primitives::blake2_256(key).to_vec(),
        };

        StorageKey(hashed)
    }

    fn block_id_by_hash(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> result::Result<BlockId<Block>, client::error::Error> {
        Ok(BlockId::Hash(
            hash.unwrap_or(self.client.info().chain.best_hash),
        ))
    }

    /// Get BlockId given the number, return the best BlockId if number is none.
    fn block_id_by_number(
        &self,
        number: Option<NumberFor<Block>>,
    ) -> result::Result<BlockId<Block>, client::error::Error> {
        let hash = match number {
            None => self.client.info().chain.best_hash,
            Some(number) => self
                .client
                .header(&BlockId::number(number))?
                .map(|h| h.hash())
                .unwrap_or(self.client.info().chain.best_hash),
        };
        Ok(BlockId::hash(hash))
    }

    fn block_number_by_hash(
        &self,
        hash: <Block as BlockT>::Hash,
    ) -> result::Result<<<Block as BlockT>::Header as HeaderT>::Number, error::Error> {
        if let Some(header) = self.client.header(&BlockId::Hash(hash))? {
            Ok(*header.number())
        } else {
            Err(error::Error::BlockNumberErr)
        }
    }

    /// Get chain state from client given the block hash.
    fn state_at(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> result::Result<
        <B as client::backend::Backend<Block, Blake2Hasher>>::State,
        client::error::Error,
    > {
        let state = self.client.state_at(&self.block_id_by_hash(hash)?)?;
        Ok(state)
    }

    /// Pick out specified data from storage given the state and key.
    fn pickout<ReturnValue: Decode>(
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        key: &[u8],
        hasher: Hasher,
    ) -> result::Result<Option<ReturnValue>, error::Error> {
        Ok(state
            .storage(&Self::storage_key(key, hasher).0)
            .map_err(|e| client::error::Error::from_state(Box::new(e)))?
            .map(StorageData)
            .map(|s| Decode::decode(&mut s.0.as_slice()))
            .unwrap_or(None))
    }

    fn try_get_v0_then_v1<V0: Decode + Default, V1: Decode + Default>(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        key: &[u8],
        key_v1: &[u8],
        hasher: Hasher,
    ) -> result::Result<std::result::Result<V0, V1>, error::Error> {
        if let Some(v) = Self::pickout::<V0>(state, key, hasher)? {
            Ok(Ok(v))
        } else if let Some(v1) = Self::pickout::<V1>(state, key_v1, hasher)? {
            Ok(Err(v1))
        } else {
            Err(error::Error::StorageNotExistErr)
        }
    }

    wrap_runtime_apis! {
        // XAssetsApi
        fn all_assets() -> Vec<(Asset, bool)>;
        fn valid_assets_of(who: AccountId) -> Vec<(Token, BTreeMap<AssetType, Balance>)>;
        fn withdrawal_list_of(chain: Chain) -> Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, Timestamp>>;
        fn deposit_list_of(chain: Chain) -> Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, Timestamp>>;
        fn withdrawal_limit(token: Token) -> Option<WithdrawalLimit<Balance>>;

        // XMiningApi
        fn asset_power(token: Token) -> Option<Balance>;
        fn jackpot_accountid_for_unsafe(who: AccountId) -> AccountId;
        fn multi_jackpot_accountid_for_unsafe(intentions: Vec<AccountId>) -> Vec<AccountId>;
        fn multi_token_jackpot_accountid_for_unsafe(tokens: Vec<Token>) -> Vec<AccountId>;

        // XSpotApi
        fn aver_asset_price(token: Token) -> Option<Balance>;

        // XFeeApi
        fn transaction_fee(power: Vec<u8>, encoded_len: u64) -> Option<u64>;

        // XStakingApi
        fn intention_set() -> Vec<AccountId>;
        fn intentions_info_common() -> Vec<xstaking::IntentionInfoCommon<AccountId, Balance, AuthorityId, BlockNumber>>;
        fn intention_info_common_of(who: &AccountId) -> Option<xstaking::IntentionInfoCommon<AccountId, Balance, AuthorityId, BlockNumber>>;

        // XBridgeApi
        fn trustee_props_for(who: AccountId) -> BTreeMap<Chain, GenericTrusteeIntentionProps>;
        fn trustee_session_info_for(chain: Chain, number: Option<u32>) -> Option<(u32, GenericAllSessionInfo<AccountId>)>;
        fn trustee_session_info() -> BTreeMap<xassets::Chain, GenericAllSessionInfo<AccountId>>;
    }

    /////////////////////////////////////////////////////////////////////////
    // Utilities for getting storage items via runtime api and some state.
    /////////////////////////////////////////////////////////////////////////

    fn get_tokens_with_jackpot_account(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: BlockId<Block>,
    ) -> result::Result<Vec<(Token, AccountId)>, error::Error> {
        let tokens = self.get_psedu_intentions(state)?;
        let jackpot_account_list =
            self.multi_token_jackpot_accountid_for_unsafe(block_id, tokens.clone())?;
        Ok(tokens
            .into_iter()
            .zip(jackpot_account_list)
            .collect::<Vec<_>>())
    }

    fn get_psedu_intention_common(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: BlockId<Block>,
        token: &Token,
        jackpot_account: AccountId,
    ) -> result::Result<PseduIntentionInfoCommon, error::Error> {
        let mut common = PseduIntentionInfoCommon::default();
        common.jackpot = self.get_pcx_free_balance(state, jackpot_account.clone())?;
        common.discount = self.get_token_discount(state, token)?;
        common.circulation = self.get_token_total_asset_balance(state, token)?;

        //注意
        //这里返回的是以PCX计价的"单位"token的价格，已含pcx精度
        //譬如1BTC=10000PCX，则返回的是10000*（10.pow(pcx精度))
        //因此，如果前端要换算折合投票数的时候
        //应该=(资产数量[含精度的数字]*price)/(10^资产精度)=PCX[含PCX精度]
        if let Ok(Some(price)) = self.aver_asset_price(block_id, token.clone()) {
            common.price = price;
        };

        if let Ok(Some(power)) = self.asset_power(block_id, token.clone()) {
            common.power = power;
        };

        common.id = to_string!(token);
        common.jackpot_account = jackpot_account.into();
        Ok(common)
    }

    /////////////////////////////////////////////////////////////////////////
    // Utilities for getting storage items via runtime api.
    /////////////////////////////////////////////////////////////////////////

    fn get_trustee_info_of(
        &self,
        block_id: BlockId<Block>,
        intention: &AccountId,
    ) -> result::Result<Vec<Chain>, error::Error> {
        let all_session_info = self.trustee_session_info(block_id)?;
        let all_trustees = all_session_info
            .into_iter()
            .map(|(chain, info)| {
                (
                    chain,
                    info.trustees_info
                        .into_iter()
                        .map(|(accountid, _)| accountid)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let mut ret = vec![];
        for (chain, trustees) in all_trustees.iter() {
            if trustees.contains(intention) {
                ret.push(*chain);
            }
        }

        Ok(ret)
    }

    /////////////////////////////////////////////////////////////////////////
    // Utilities for getting storage items from a certain state.
    /////////////////////////////////////////////////////////////////////////

    /// Get all tokens, i.e., psedu intntions.
    fn get_psedu_intentions(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
    ) -> result::Result<Vec<Token>, error::Error> {
        let key = <xtokens::PseduIntentions<Runtime>>::key();
        Ok(Self::pickout::<Vec<Token>>(state, &key, Hasher::TWOX128)?.unwrap_or_default())
    }

    fn get_token_free_balance(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        who: AccountId,
        token: Token,
    ) -> result::Result<Balance, error::Error> {
        let key = (who, token);
        let balances_key = <xassets::AssetBalance<Runtime>>::key_for(&key);
        let map =
            Self::pickout::<BTreeMap<AssetType, Balance>>(state, &balances_key, Hasher::BLAKE2256)?
                .unwrap_or_default();
        Ok(map.get(&AssetType::Free).copied().unwrap_or_default())
    }

    /// Get free balance of PCX given an account.
    fn get_pcx_free_balance(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        who: AccountId,
    ) -> result::Result<Balance, error::Error> {
        self.get_token_free_balance(state, who, xassets::Module::<Runtime>::TOKEN.to_vec())
    }

    fn get_intention_jackpot_balance(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: BlockId<Block>,
        intention: AccountId,
    ) -> result::Result<Balance, error::Error> {
        let jackpot_account = self.jackpot_accountid_for_unsafe(block_id, intention)?;
        self.get_pcx_free_balance(state, jackpot_account)
    }

    fn get_psedu_intention_jackpot_balance(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: BlockId<Block>,
        token: Token,
    ) -> result::Result<Balance, error::Error> {
        let jackpot_accounts =
            self.multi_token_jackpot_accountid_for_unsafe(block_id, vec![token])?;
        self.get_pcx_free_balance(state, jackpot_accounts[0].clone())
    }

    fn sum_of_all_kinds_of_balance(balances: BTreeMap<AssetType, Balance>) -> Balance {
        balances.iter().fold(Zero::zero(), |acc, (_, v)| acc + *v)
    }

    /// Get total balance of all kinds of some token.
    fn get_token_total_asset_balance(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        token: &Token,
    ) -> result::Result<Balance, error::Error> {
        let key = <xassets::TotalAssetBalance<Runtime>>::key_for(token);
        Ok(
            Self::pickout::<BTreeMap<AssetType, Balance>>(state, &key, Hasher::BLAKE2256)?
                .map(Self::sum_of_all_kinds_of_balance)
                .unwrap_or_default(),
        )
    }

    /// Get total balance of account given the token type.
    fn get_total_asset_balance_of(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        wt_key: &(AccountId, Token),
    ) -> result::Result<Balance, error::Error> {
        let key = <xassets::AssetBalance<Runtime>>::key_for(wt_key);
        Ok(
            Self::pickout::<BTreeMap<AssetType, Balance>>(state, &key, Hasher::BLAKE2256)?
                .map(Self::sum_of_all_kinds_of_balance)
                .unwrap_or_default(),
        )
    }

    fn get_events(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
    ) -> result::Result<
        Vec<
            system::EventRecord<
                <chainx_runtime::Runtime as system::Trait>::Event,
                <chainx_runtime::Runtime as system::Trait>::Hash,
            >,
        >,
        error::Error,
    > {
        let key = b"System Events";
        Ok(Self::pickout(state, &key[..], Hasher::TWOX128)?.unwrap_or_default())
    }

    fn get_token_discount(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        token: &Token,
    ) -> result::Result<u32, error::Error> {
        let key = <xtokens::TokenDiscount<Runtime>>::key_for(token);
        Ok(Self::pickout::<u32>(state, &key, Hasher::BLAKE2256)?.unwrap_or_default())
    }

    fn get_next_claim(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        who: AccountId,
        token: &Token,
    ) -> result::Result<BlockNumber, error::Error> {
        let key = <xtokens::ClaimRestrictionOf<Runtime>>::key_for(token);
        let (_, interval) = Self::pickout::<(u32, BlockNumber)>(state, &key, Hasher::BLAKE2256)?
            .unwrap_or((10u32, xtokens::BLOCKS_PER_WEEK));

        let key = <xtokens::LastClaimOf<Runtime>>::key_for(&(who, token.clone()));

        Ok(
            Self::pickout::<BlockNumber>(state, &key, Hasher::BLAKE2256)?
                .map(|last_claim| last_claim + interval)
                .unwrap_or_default(),
        )
    }

    fn try_get_nomination_record(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        nr_key: &(AccountId, AccountId),
    ) -> result::Result<
        std::result::Result<
            xstaking::NominationRecord<Balance, BlockNumber>,
            xstaking::NominationRecordV1<Balance, BlockNumber>,
        >,
        error::Error,
    > {
        let key = <xstaking::NominationRecords<Runtime>>::key_for(nr_key);
        let key_v1 = <xstaking::NominationRecordsV1<Runtime>>::key_for(nr_key);
        self.try_get_v0_then_v1::<
            xstaking::NominationRecord<Balance, BlockNumber>,
            xstaking::NominationRecordV1<Balance, BlockNumber>
        >(state, &key, &key_v1, Hasher::BLAKE2256)
    }

    fn get_nomination_records_wrapper(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> result::Result<Vec<(AccountId, NominationRecordWrapper)>, error::Error> {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;
        let who: AccountId = who.unchecked_into();

        let records = lru_cache!(AccountId, Vec<(AccountId, NominationRecordWrapper)>, size=512; key=who; hash; self {
        let mut records = Vec::new();
        for intention in self.intention_set(block_id)? {
            let nr_key = (who.clone(), intention.clone());
            if let Ok(record) = self.try_get_nomination_record(&state, &nr_key) {
                records.push((intention, NominationRecordWrapper(record)));
            }
        }
        records
        });
        Ok(records)
    }

    fn get_psedu_nomination_record_common(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        who: &AccountId,
        token: &Token,
    ) -> result::Result<PseduNominationRecordCommon, error::Error> {
        let mut common = PseduNominationRecordCommon::default();
        common.id = to_string!(token);
        common.balance = self.get_total_asset_balance_of(state, &(who.clone(), token.clone()))?;
        common.next_claim = self.get_next_claim(state, who.clone(), token)?;
        Ok(common)
    }

    fn try_get_intention_profs(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        intention: &AccountId,
    ) -> result::Result<
        std::result::Result<
            xstaking::IntentionProfs<Balance, BlockNumber>,
            xstaking::IntentionProfsV1<Balance, BlockNumber>,
        >,
        error::Error,
    > {
        let key = <xstaking::Intentions<Runtime>>::key_for(intention);
        let key_v1 = <xstaking::IntentionsV1<Runtime>>::key_for(intention);
        self.try_get_v0_then_v1::<
            xstaking::IntentionProfs<Balance, BlockNumber>,
            xstaking::IntentionProfsV1<Balance, BlockNumber>
        >(state, &key, &key_v1, Hasher::BLAKE2256)
    }

    fn into_or_get_intention_profs_v1(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        intention: &AccountId,
    ) -> result::Result<xstaking::IntentionProfsV1<Balance, BlockNumber>, error::Error> {
        match self.try_get_intention_profs(state, intention)? {
            Ok(x) => Ok(x.into()),
            Err(x) => Ok(x),
        }
    }

    fn into_intention_info_wrapper(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: (BlockId<Block>, Option<<Block as BlockT>::Hash>),
        common_info: xstaking::IntentionInfoCommon<AccountId, Balance, AuthorityId, BlockNumber>,
    ) -> result::Result<IntentionInfoWrapper, error::Error> {
        let who = common_info.account.clone();
        let hash = block_id.1;
        let info = lru_cache!(AccountId, IntentionInfoWrapper, size=512; key=who; hash; self {
        let is_trustee = self.get_trustee_info_of(block_id.0, &who)?;
        let intention_profs_wrapper = self.try_get_intention_profs(state, &who)?;

        let intention_props =
            IntentionPropsForRpc::new(common_info.intention_props.clone(), who.clone());

        IntentionInfoWrapper {
            intention_common: IntentionInfoCommon {
                common: common_info.into(),
                is_trustee,
                intention_props,
            },
            intention_profs_wrapper,
        }
        });
        Ok(info)
    }

    fn get_intention_info_wrapper(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: (BlockId<Block>, Option<<Block as BlockT>::Hash>),
        who: AccountId,
    ) -> result::Result<Option<IntentionInfoWrapper>, error::Error> {
        let result = if let Some(common_info) = self.intention_info_common_of(block_id.0, &who)? {
            Some(self.into_intention_info_wrapper(state, block_id, common_info)?)
        } else {
            None
        };
        Ok(result)
    }

    fn get_intentions_info_wrapper(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: (BlockId<Block>, Option<<Block as BlockT>::Hash>),
    ) -> result::Result<Vec<IntentionInfoWrapper>, error::Error> {
        let mut intentions_info = Vec::new();
        for common_info in self.intentions_info_common(block_id.0)? {
            intentions_info.push(self.into_intention_info_wrapper(state, block_id, common_info)?);
        }
        Ok(intentions_info)
    }

    fn try_get_psedu_intention_profs(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        token: &Token,
    ) -> result::Result<
        std::result::Result<
            xtokens::PseduIntentionVoteWeight<Balance>,
            xtokens::PseduIntentionVoteWeightV1<Balance>,
        >,
        error::Error,
    > {
        let key = <PseduIntentionProfiles<Runtime>>::key_for(token);
        let key_v1 = <PseduIntentionProfilesV1<Runtime>>::key_for(token);
        self.try_get_v0_then_v1::<
            PseduIntentionVoteWeight<Balance>,
            PseduIntentionVoteWeightV1<Balance>
        >(state, &key, &key_v1, Hasher::BLAKE2256)
    }

    fn into_or_get_psedu_intention_profs_v1(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        token: &Token,
    ) -> result::Result<xtokens::PseduIntentionVoteWeightV1<Balance>, error::Error> {
        match self.try_get_psedu_intention_profs(state, token)? {
            Ok(x) => Ok(x.into()),
            Err(x) => Ok(x),
        }
    }

    fn get_psedu_intentions_info_wrapper(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        block_id: BlockId<Block>,
    ) -> result::Result<Vec<PseduIntentionInfoWrapper>, error::Error> {
        let mut psedu_intentions_info = Vec::new();

        for (token, jackpot_account) in self.get_tokens_with_jackpot_account(state, block_id)? {
            if let Ok(psedu_intention_profs_wrapper) =
                self.try_get_psedu_intention_profs(state, &token)
            {
                psedu_intentions_info.push(PseduIntentionInfoWrapper {
                    psedu_intention_common: self.get_psedu_intention_common(
                        state,
                        block_id,
                        &token,
                        jackpot_account,
                    )?,
                    psedu_intention_profs_wrapper,
                });
            }
        }
        Ok(psedu_intentions_info)
    }

    fn try_get_deposit_vote_weight(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        wt_key: &(AccountId, Token),
    ) -> result::Result<
        std::result::Result<
            xtokens::DepositVoteWeight<Balance>,
            xtokens::DepositVoteWeightV1<Balance>,
        >,
        error::Error,
    > {
        let key = <DepositRecords<Runtime>>::key_for(wt_key);
        let key_v1 = <DepositRecordsV1<Runtime>>::key_for(wt_key);
        self.try_get_v0_then_v1::<DepositVoteWeight<BlockNumber>, DepositVoteWeightV1<BlockNumber>>(
            state,
            &key,
            &key_v1,
            Hasher::BLAKE2256,
        )
    }

    fn get_psedu_nomination_records_wrapper(
        &self,
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        who: AccountId,
    ) -> result::Result<Vec<PseduNominationRecordWrapper>, error::Error> {
        let mut psedu_records = Vec::new();
        for token in self.get_psedu_intentions(&state)? {
            if let Ok(deposit_vote_weight_wrapper) =
                self.try_get_deposit_vote_weight(&state, &(who.clone(), token.clone()))
            {
                psedu_records.push(PseduNominationRecordWrapper {
                    common: self.get_psedu_nomination_record_common(&state, &who, &token)?,
                    deposit_vote_weight_wrapper,
                });
            }
        }
        Ok(psedu_records)
    }
}
