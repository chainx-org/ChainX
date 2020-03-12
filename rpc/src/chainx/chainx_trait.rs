use super::*;

/// ChainX API
#[rpc]
pub trait ChainXApi<Number, Hash, AccountId, Balance, BlockNumber, SignedBlock> {
    /// Returns the block of a storage entry at a block's Number.
    #[rpc(name = "chainx_getBlockByNumber")]
    fn block_info(&self, number: Option<Number>) -> Result<Option<SignedBlock>>;

    #[rpc(name = "chainx_getExtrinsicsEventsByBlockHash")]
    fn extrinsics_events(&self, hash: Option<Hash>) -> Result<Value>;

    #[rpc(name = "chainx_getEventsByBlockHash")]
    fn events(&self, hash: Option<Hash>) -> Result<Value>;

    #[rpc(name = "chainx_getNextRenominateByAccount")]
    fn next_renominate(&self, who: AccountId, hash: Option<Hash>) -> Result<Option<BlockNumber>>;

    #[rpc(name = "chainx_getStakingDividendByAccount")]
    fn staking_dividend(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<BTreeMap<AccountIdForRpc, Balance>>;

    #[rpc(name = "chainx_getCrossMiningDividendByAccount")]
    fn cross_mining_dividend(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<BTreeMap<String, Value>>;

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
    ) -> Result<Option<Vec<(AccountId, NominationRecordForRpc)>>>;

    #[rpc(name = "chainx_getNominationRecordsV1")]
    fn nomination_records_v1(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<Option<Vec<(AccountId, NominationRecordV1ForRpc)>>>;

    #[rpc(name = "chainx_getIntentions")]
    fn intentions(&self, hash: Option<Hash>) -> Result<Option<Vec<IntentionInfo>>>;

    #[rpc(name = "chainx_getIntentionsV1")]
    fn intentions_v1(&self, hash: Option<Hash>) -> Result<Option<Vec<IntentionInfoV1>>>;

    #[rpc(name = "chainx_getIntentionByAccount")]
    fn intention(&self, who: AccountId, hash: Option<Hash>) -> Result<Option<IntentionInfo>>;

    #[rpc(name = "chainx_getIntentionByAccountV1")]
    fn intention_v1(&self, who: AccountId, hash: Option<Hash>) -> Result<Option<IntentionInfoV1>>;

    #[rpc(name = "chainx_getPseduIntentions")]
    fn psedu_intentions(&self, hash: Option<Hash>) -> Result<Option<Vec<PseduIntentionInfo>>>;

    #[rpc(name = "chainx_getPseduIntentionsV1")]
    fn psedu_intentions_v1(&self, hash: Option<Hash>) -> Result<Option<Vec<PseduIntentionInfoV1>>>;

    #[rpc(name = "chainx_getPseduNominationRecords")]
    fn psedu_nomination_records(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<Option<Vec<PseduNominationRecord>>>;

    #[rpc(name = "chainx_getPseduNominationRecordsV1")]
    fn psedu_nomination_records_v1(
        &self,
        who: AccountId,
        hash: Option<Hash>,
    ) -> Result<Option<Vec<PseduNominationRecordV1>>>;

    #[rpc(name = "chainx_getTradingPairs")]
    fn trading_pairs(&self, hash: Option<Hash>) -> Result<Option<Vec<PairInfo>>>;

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

    #[rpc(name = "chainx_getFeeWeightMap")]
    fn fee_weight_map(&self, hash: Option<Hash>) -> Result<Value>;

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

    #[rpc(name = "chainx_contractCall")]
    fn contract_call(&self, call_request: CallRequest, at: Option<Hash>) -> Result<Value>;

    #[rpc(name = "chainx_contractGetStorage")]
    fn contract_get_storage(
        &self,
        address: AccountId,
        key: H256,
        at: Option<Hash>,
    ) -> Result<Option<Bytes>>;

    #[rpc(name = "chainx_contractXRC20Call")]
    fn contract_xrc20_call(
        &self,
        call_request: XRC20CallRequest,
        at: Option<Hash>,
    ) -> Result<Value>;

    #[rpc(name = "chainx_contractXRCTokenInfo")]
    fn contract_xrc_token_info(&self, at: Option<Hash>) -> Result<BTreeMap<String, Value>>;
}
