// Copyright 2018-2019 Chainpool.

use std::collections::btree_map::BTreeMap;
use std::result;
use std::sync::Arc;

use jsonrpc_derive::rpc;
use parity_codec::Decode;
use serde_json::Value;

use chainx_primitives::{AccountId, Balance, BlockNumber, Timestamp};
use client::runtime_api::Metadata;
use primitives::storage::{StorageData, StorageKey};
use primitives::{Blake2Hasher, H256};
use runtime_primitives::generic::BlockId;
use runtime_primitives::traits::Block as BlockT;
use runtime_primitives::traits::{Header, NumberFor, ProvideRuntimeApi};
use state_machine::Backend;

use runtime_api::{
    xassets_api::XAssetsApi, xbridge_api::XBridgeApi, xfee_api::XFeeApi, xmining_api::XMiningApi,
    xspot_api::XSpotApi, xstaking_api::XStakingApi,
};

use xassets::{Asset, AssetType, Chain, Token};
use xbridge_common::types::{GenericAllSessionInfo, GenericTrusteeIntentionProps};
use xprocess::WithdrawalLimit;
use xspot::TradingPairIndex;

mod error;
mod impl_rpc;
mod types;
mod utils;

use self::error::Result;
pub use self::types::*;

/// ChainX API
#[rpc]
pub trait ChainXApi<Number, Hash, AccountId, Balance, BlockNumber, SignedBlock> {
    /// Returns the block of a storage entry at a block's Number.
    #[rpc(name = "chainx_getBlockByNumber")]
    fn block_info(&self, number: Option<Number>) -> Result<Option<SignedBlock>>;

    #[rpc(name = "chainx_getAssetsByAccount")]
    fn assets_of(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
        hash: Option<Hash>,
    ) -> Result<Option<PageData<AssetInfo>>>;

    #[rpc(name = "chainx_getAssets")]
    fn assets(
        &self,
        page_index: u32,
        page_size: u32,
        hash: Option<Hash>,
    ) -> Result<Option<PageData<TotalAssetInfo>>>;

    #[rpc(name = "chainx_verifyAddressValidity")]
    fn verify_addr(
        &self,
        token: String,
        addr: String,
        memo: String,
        hash: Option<Hash>,
    ) -> Result<Option<bool>>;

    #[rpc(name = "chainx_getWithdrawalLimitByToken")]
    fn withdrawal_limit(
        &self,
        token: String,
        hash: Option<Hash>,
    ) -> Result<Option<WithdrawalLimit<Balance>>>;

    #[rpc(name = "chainx_getDepositLimitByToken")]
    fn deposit_limit(&self, token: String, hash: Option<Hash>) -> Result<Option<DepositLimit>>;

    #[rpc(name = "chainx_getDepositList")]
    fn deposit_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
        hash: Option<Hash>,
    ) -> Result<Option<PageData<DepositInfo>>>;

    #[rpc(name = "chainx_getWithdrawalList")]
    fn withdrawal_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
        hash: Option<Hash>,
    ) -> Result<Option<PageData<WithdrawInfo>>>;

    #[rpc(name = "chainx_getNominationRecords")]
    fn nomination_records(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<Option<Vec<(AccountId, NominationRecord)>>>;

    #[rpc(name = "chainx_getIntentions")]
    fn intentions(&self, hash: Option<Hash>) -> Result<Option<Vec<IntentionInfo>>>;

    #[rpc(name = "chainx_getIntentionByAccount")]
    fn intention(&self, who: AccountId, hash: Option<Hash>) -> Result<Option<Value>>;

    #[rpc(name = "chainx_getPseduIntentions")]
    fn psedu_intentions(&self, hash: Option<Hash>) -> Result<Option<Vec<PseduIntentionInfo>>>;

    #[rpc(name = "chainx_getPseduNominationRecords")]
    fn psedu_nomination_records(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<Option<Vec<PseduNominationRecord>>>;

    #[rpc(name = "chainx_getTradingPairs")]
    fn trading_pairs(&self, hash: Option<Hash>) -> Result<Option<Vec<(PairInfo)>>>;

    #[rpc(name = "chainx_getQuotations")]
    fn quotations(
        &self,
        id: TradingPairIndex,
        piece: u32,
        hash: Option<Hash>,
    ) -> Result<Option<QuotationsList>>;

    #[rpc(name = "chainx_getOrders")]
    fn orders(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
        hash: Option<Hash>,
    ) -> Result<Option<PageData<OrderDetails>>>;

    #[rpc(name = "chainx_getAddressByAccount")]
    fn address(
        &self,
        who: AccountId,
        chain: Chain,
        hash: Option<Hash>,
    ) -> Result<Option<Vec<String>>>;

    #[rpc(name = "chainx_getTrusteeSessionInfo")]
    fn trustee_session_info_for(
        &self,
        chain: Chain,
        number: Option<u32>,
        hash: Option<Hash>,
    ) -> Result<Option<Value>>;

    #[rpc(name = "chainx_getTrusteeInfoByAccount")]
    fn trustee_info_for_accountid(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<Option<Value>>;

    #[rpc(name = "chainx_getFeeByCallAndLength")]
    fn fee(&self, call_params: String, tx_length: u64, hash: Option<Hash>) -> Result<Option<u64>>;

    #[rpc(name = "chainx_getWithdrawTx")]
    fn withdraw_tx(&self, chain: Chain, hash: Option<Hash>) -> Result<Option<WithdrawTxInfo>>;

    #[rpc(name = "chainx_getMockBitcoinNewTrustees")]
    fn mock_bitcoin_new_trustees(
        &self,
        candidates: Vec<AccountId>,
        hash: Option<Hash>,
    ) -> Result<Option<Value>>;

    #[rpc(name = "chainx_particularAccounts")]
    fn particular_accounts(&self, hash: Option<Hash>) -> Result<Option<serde_json::Value>>;
}

/// Wrap runtime apis in ChainX API.
macro_rules! wrap_runtime_apis {
    (
        $(
            fn $fn_name:ident( $($arg:ident : $arg_ty:ty),* ) -> $ret:ty;
        )+
    ) => {
        $(
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
        + XBridgeApi<Block>,
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
            hash.unwrap_or(self.client.info()?.chain.best_hash),
        ))
    }

    /// Get BlockId given the number, return the best BlockId if number is none.
    fn block_id_by_number(
        &self,
        number: Option<NumberFor<Block>>,
    ) -> result::Result<BlockId<Block>, client::error::Error> {
        let hash = match number {
            None => self.client.info()?.chain.best_hash,
            Some(number) => self
                .client
                .header(&BlockId::number(number))?
                .map(|h| h.hash())
                .unwrap_or(self.client.info()?.chain.best_hash),
        };
        Ok(BlockId::Hash(hash))
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
            .map_err(|e| error::Error::from_state(Box::new(e)))?
            .map(StorageData)
            .map(|s| Decode::decode(&mut s.0.as_slice()))
            .unwrap_or(None))
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

        // XBridgeApi
        fn trustee_props_for(who: AccountId) -> BTreeMap<Chain, GenericTrusteeIntentionProps>;
        fn trustee_session_info_for(chain: Chain, number: Option<u32>) -> Option<(u32, GenericAllSessionInfo<AccountId>)>;
        fn trustee_session_info() -> BTreeMap<xassets::Chain, GenericAllSessionInfo<AccountId>>;
    }
}
