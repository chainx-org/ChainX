// Copyright 2019 Chainpool.

use std::collections::BTreeMap;
use std::result::Result as StdResult;
use std::sync::Arc;

use jsonrpc_derive::rpc;
use parity_codec::Decode;

use primitives::storage::{StorageData, StorageKey};
use primitives::{Blake2Hasher, H256};
use runtime_primitives::generic::{BlockId, SignedBlock};
use runtime_primitives::traits::{As, Block as BlockT, NumberFor, Zero};
use state_machine::Backend;

use chainx_primitives::{AccountId, AccountIdForRpc, AuthorityId, Balance, BlockNumber, Timestamp};
use chainx_runtime::{Call, Runtime};

use xaccounts::{IntentionProps, TrusteeEntity, TrusteeIntentionProps, TrusteeSessionInfo};
use xassets::{Asset, AssetType, Chain, Token};
use xbitcoin::{TrusteeAddrInfo as BtcTrusteeAddrInfo, VoteResult};
use xrecords::{RecordInfo, TxState};
use xspot::{
    HandicapInfo, OrderDirection, OrderIndex, OrderInfo, OrderStatus, OrderType, TradeHistoryIndex,
    TradingPair, TradingPairIndex,
};
use xstaking::IntentionProfs;
use xsupport::storage::btree_map::CodecBTreeMap;
use xtokens::{DepositVoteWeight, PseduIntentionVoteWeight};

mod error;
mod impl_rpc;
pub mod types;

use self::error::{ErrorKind, Result};
use self::types::*;

const MAX_PAGE_SIZE: u32 = 100;

/// ChainX API
#[rpc]
pub trait ChainXApi<Number, AccountId, Balance, BlockNumber, SignedBlock> {
    /// Returns the block of a storage entry at a block's Number.
    #[rpc(name = "chainx_getBlockByNumber")]
    fn block_info(&self, number: Option<Number>) -> Result<Option<SignedBlock>>;

    #[rpc(name = "chainx_getAssetsByAccount")]
    fn assets_of(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<AssetInfo>>>;

    #[rpc(name = "chainx_getAssets")]
    fn assets(&self, page_index: u32, page_size: u32) -> Result<Option<PageData<TotalAssetInfo>>>;

    #[rpc(name = "chainx_verifyAddressValidity")]
    fn verify_addr(&self, token: String, addr: String, memo: String) -> Result<Option<bool>>;

    #[rpc(name = "chainx_getMinimalWithdrawalValueByToken")]
    fn minimal_withdrawal_value(&self, token: String) -> Result<Option<Balance>>;

    #[rpc(name = "chainx_getDepositList")]
    fn deposit_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<DepositInfo>>>;

    #[rpc(name = "chainx_getWithdrawalList")]
    fn withdrawal_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<WithdrawInfo>>>;

    #[rpc(name = "chainx_getNominationRecords")]
    fn nomination_records(
        &self,
        who: AccountId,
    ) -> Result<Option<Vec<(AccountId, NominationRecord)>>>;

    #[rpc(name = "chainx_getIntentions")]
    fn intentions(&self) -> Result<Option<Vec<IntentionInfo>>>;

    #[rpc(name = "chainx_getPseduIntentions")]
    fn psedu_intentions(&self) -> Result<Option<Vec<PseduIntentionInfo>>>;

    #[rpc(name = "chainx_getPseduNominationRecords")]
    fn psedu_nomination_records(
        &self,
        who: AccountId,
    ) -> Result<Option<Vec<PseduNominationRecord>>>;

    #[rpc(name = "chainx_getOrderPairs")]
    fn order_pairs(&self) -> Result<Option<Vec<(PairInfo)>>>;

    #[rpc(name = "chainx_getQuotations")]
    fn quotationss(&self, id: TradingPairIndex, piece: u32) -> Result<Option<QuotationsList>>;

    #[rpc(name = "chainx_getOrders")]
    fn orders(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<OrderDetails>>>;

    #[rpc(name = "chainx_getAddressByAccount")]
    fn address(&self, who: AccountId, chain: Chain) -> Result<Option<Vec<String>>>;

    #[rpc(name = "chainx_getTrusteeSessionInfo")]
    fn trustee_session_info(&self, chain: Chain) -> Result<Option<CurrentTrusteeSessionInfo>>;

    #[rpc(name = "chainx_getTrusteeInfoByAccount")]
    fn trustee_info_for_accountid(&self, who: AccountId) -> Result<Vec<TrusteeInfo>>;

    #[rpc(name = "chainx_getFeeByCallAndLength")]
    fn fee(&self, call_params: String, tx_length: u64) -> Result<Option<u64>>;

    #[rpc(name = "chainx_getWithdrawTx")]
    fn withdraw_tx(&self, chain: Chain) -> Result<Option<WithdrawTxInfo>>;

    #[rpc(name = "chainx_getMockBitcoinNewTrustees")]
    fn mock_bitcoin_new_trustees(
        &self,
        candidates: Vec<AccountId>,
    ) -> Result<Option<MockBitcoinTrustee>>;
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
{
    /// Create new ChainX API RPC handler.
    pub fn new(client: Arc<client::Client<B, E, Block, RA>>) -> Self {
        Self { client }
    }

    /// Generate storage key.
    fn storage_key(key: &[u8]) -> StorageKey {
        let hashed = primitives::twox_128(key).to_vec();
        StorageKey(hashed)
    }

    /// Get best number of the chain.
    fn best_number(&self) -> StdResult<BlockId<Block>, client::error::Error> {
        let best_hash = self.client.info()?.chain.best_hash;
        Ok(BlockId::Hash(best_hash))
    }

    /// Get state of best number of the chain.
    fn best_state(
        &self,
    ) -> StdResult<<B as client::backend::Backend<Block, Blake2Hasher>>::State, client::error::Error>
    {
        let state = self.client.state_at(&self.best_number()?)?;
        Ok(state)
    }

    /// Pick out specified data from storage given the state and key.
    fn pickout<ReturnValue: Decode>(
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        key: &[u8],
    ) -> StdResult<Option<ReturnValue>, error::Error> {
        Ok(state
            .storage(&Self::storage_key(key).0)
            .map_err(|e| error::Error::from_state(Box::new(e)))?
            .map(StorageData)
            .map(|s| Decode::decode(&mut s.0.as_slice()))
            .unwrap_or(None))
    }

    fn current_trustee_session_info(
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        chain: Chain,
    ) -> Result<Option<(TrusteeSessionInfo<AccountId>, u32)>> {
        use support::storage::generator::StorageMap;
        let key = <xaccounts::TrusteeSessionInfoLen<Runtime>>::key_for(&chain);
        let session_length = Self::pickout::<u32>(state, &key)?.unwrap_or_default();
        let session_number = if session_length == 0 {
            u32::max_value()
        } else {
            session_length - 1
        };
        let key = <xaccounts::TrusteeSessionInfoOf<Runtime>>::key_for(&(chain, session_number));

        Self::pickout::<TrusteeSessionInfo<AccountId>>(state, &key)
            .map(|item_option| item_option.map(|item| (item, session_number)))
    }
}
