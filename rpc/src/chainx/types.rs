// Copyright 2018-2019 Chainpool.

use super::*;

use std::convert::From;

use rustc_hex::ToHex;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use btc_ser::serialize as btc_serialize;

// chainx
use chainx_primitives::AccountIdForRpc;

use xassets::AssetLimit;
use xbitcoin::VoteResult;
use xrecords::{HeightOrTime, RecordInfo, TxState};
use xspot::{
    OrderIndex, OrderInfo, OrderStatus, OrderType, Side, TradeHistoryIndex, TradingPairIndex,
};

pub const MAX_PAGE_SIZE: u32 = 100;

#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum Hasher {
    TWOX128,
    BLAKE2256,
}
impl Default for Hasher {
    fn default() -> Self {
        Hasher::TWOX128
    }
}

// utils
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData<T> {
    pub page_total: u32,
    pub page_index: u32,
    pub page_size: u32,
    pub data: Vec<T>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    pub name: String,
    pub details: BTreeMap<AssetType, Balance>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalAssetInfo {
    pub name: String,
    pub token_name: String,
    pub chain: Chain,
    pub precision: u16,
    pub desc: String,
    pub online: bool,
    pub details: BTreeMap<AssetType, Balance>,
    pub limit_props: BTreeMap<AssetLimit, bool>,
}

impl TotalAssetInfo {
    pub fn new(
        asset: Asset,
        online: bool,
        details: BTreeMap<AssetType, Balance>,
        limit_props: BTreeMap<AssetLimit, bool>,
    ) -> TotalAssetInfo {
        TotalAssetInfo {
            name: String::from_utf8_lossy(&asset.token()).into_owned(),
            token_name: String::from_utf8_lossy(&asset.token_name()).into_owned(),
            chain: asset.chain(),
            precision: asset.precision(),
            desc: String::from_utf8_lossy(&asset.desc()).into_owned(),
            online,
            details,
            limit_props,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revocation {
    pub block_number: BlockNumber,
    pub value: Balance,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NominationRecordForRpc {
    pub nomination: Balance,
    pub last_vote_weight: u64,
    pub last_vote_weight_update: BlockNumber,
    pub revocations: Vec<Revocation>,
}

#[inline]
fn to_revocation_struct(revocations: Vec<(BlockNumber, Balance)>) -> Vec<Revocation> {
    revocations
        .iter()
        .map(|x| Revocation {
            block_number: x.0,
            value: x.1,
        })
        .collect::<Vec<_>>()
}

impl From<xstaking::NominationRecord<Balance, BlockNumber>> for NominationRecordForRpc {
    fn from(record: xstaking::NominationRecord<Balance, BlockNumber>) -> Self {
        Self {
            nomination: record.nomination,
            last_vote_weight: record.last_vote_weight,
            last_vote_weight_update: record.last_vote_weight_update,
            revocations: to_revocation_struct(record.revocations),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NominationRecordV1ForRpc {
    pub nomination: Balance,
    pub last_vote_weight: String,
    pub last_vote_weight_update: BlockNumber,
    pub revocations: Vec<Revocation>,
}

impl From<xstaking::NominationRecordV1<Balance, BlockNumber>> for NominationRecordV1ForRpc {
    fn from(record: xstaking::NominationRecordV1<Balance, BlockNumber>) -> Self {
        Self {
            nomination: record.nomination,
            last_vote_weight: format!("{}", record.last_vote_weight),
            last_vote_weight_update: record.last_vote_weight_update,
            revocations: to_revocation_struct(record.revocations),
        }
    }
}

/// Intention properties
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionPropsForRpc {
    /// url
    pub url: String,
    /// is running for the validators
    pub is_active: bool,
    /// about
    pub about: String,
    /// session key for block authoring
    pub session_key: AccountIdForRpc,
}

impl IntentionPropsForRpc {
    pub fn new(
        props: xaccounts::IntentionProps<AuthorityId, BlockNumber>,
        intention: AccountId,
    ) -> Self {
        Self {
            url: String::from_utf8_lossy(&props.url).into_owned(),
            is_active: props.is_active,
            about: String::from_utf8_lossy(&props.about).into_owned(),
            session_key: props.session_key.unwrap_or(intention).into(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoCommon {
    #[serde(flatten)]
    pub common: IntentionInfoCommonForRpc,
    #[serde(flatten)]
    pub intention_props: IntentionPropsForRpc,
    /// is trustee
    pub is_trustee: Vec<Chain>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoCommonForRpc {
    /// account id of intention
    pub account: AccountIdForRpc,
    /// name of intention
    pub name: String,
    /// is validator
    pub is_validator: bool,
    /// how much has intention voted for itself
    pub self_vote: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// jackpot account
    pub jackpot_account: AccountIdForRpc,
}

impl From<xstaking::IntentionInfoCommon<AccountId, Balance>> for IntentionInfoCommonForRpc {
    fn from(common: xstaking::IntentionInfoCommon<AccountId, Balance>) -> Self {
        Self {
            account: common.account.into(),
            name: String::from_utf8_lossy(&common.name.unwrap_or_default()).into_owned(),
            is_validator: common.is_validator,
            self_vote: common.self_bonded,
            jackpot: common.jackpot_balance,
            jackpot_account: common.jackpot_account.into(),
        }
    }
}

/// Due to the serde inability about u128, we use String instead of u128 here.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionProfsV1ForRpc {
    /// total nomination from all nominators
    pub total_nomination: Balance,
    /// vote weight at last update
    pub last_total_vote_weight: String,
    /// last update time of vote weight
    pub last_total_vote_weight_update: BlockNumber,
}

impl From<xstaking::IntentionProfsV1<Balance, BlockNumber>> for IntentionProfsV1ForRpc {
    fn from(iprof_v1: xstaking::IntentionProfsV1<Balance, BlockNumber>) -> Self {
        Self {
            total_nomination: iprof_v1.total_nomination,
            last_total_vote_weight: format!("{}", iprof_v1.last_total_vote_weight),
            last_total_vote_weight_update: iprof_v1.last_total_vote_weight_update,
        }
    }
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfo {
    #[serde(flatten)]
    pub intention_common: IntentionInfoCommon,
    #[serde(flatten)]
    pub intention_profs: xstaking::IntentionProfs<Balance, BlockNumber>,
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoV1 {
    #[serde(flatten)]
    pub intention_common: IntentionInfoCommon,
    #[serde(flatten)]
    pub intention_profs: IntentionProfsV1ForRpc,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoWrapper {
    pub intention_common: IntentionInfoCommon,
    pub intention_profs_wrapper: result::Result<
        xstaking::IntentionProfs<Balance, BlockNumber>,
        xstaking::IntentionProfsV1<Balance, BlockNumber>,
    >,
}

impl From<IntentionInfoWrapper> for IntentionInfo {
    fn from(info_wrapper: IntentionInfoWrapper) -> Self {
        Self {
            intention_common: info_wrapper.intention_common,
            intention_profs: match info_wrapper.intention_profs_wrapper {
                Ok(x) => x,
                Err(_) => unreachable!("Ensured it's Ok"),
            },
        }
    }
}

impl From<IntentionInfoWrapper> for IntentionInfoV1 {
    fn from(info_wrapper: IntentionInfoWrapper) -> Self {
        Self {
            intention_common: info_wrapper.intention_common,
            intention_profs: match info_wrapper.intention_profs_wrapper {
                Ok(x) => {
                    let x: xstaking::IntentionProfsV1<Balance, BlockNumber> = x.into();
                    x.into()
                }
                Err(x) => x.into(),
            },
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfo {
    #[serde(flatten)]
    pub psedu_intention_common: PseduIntentionInfoCommon,
    #[serde(flatten)]
    pub psedu_intention_profs: xtokens::PseduIntentionVoteWeight<Balance>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfoV1 {
    #[serde(flatten)]
    pub psedu_intention_common: PseduIntentionInfoCommon,
    #[serde(flatten)]
    pub psedu_intention_profs: PseduIntentionVoteWeightV1ForRpc,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfoWrapper {
    pub psedu_intention_common: PseduIntentionInfoCommon,
    pub psedu_intention_profs_wrapper: result::Result<
        xtokens::PseduIntentionVoteWeight<Balance>,
        xtokens::PseduIntentionVoteWeightV1<Balance>,
    >,
}

impl From<PseduIntentionInfoWrapper> for PseduIntentionInfo {
    fn from(info_wrapper: PseduIntentionInfoWrapper) -> Self {
        Self {
            psedu_intention_common: info_wrapper.psedu_intention_common,
            psedu_intention_profs: match info_wrapper.psedu_intention_profs_wrapper {
                Ok(x) => x,
                Err(_) => unreachable!("Ensured it's Ok"),
            },
        }
    }
}

impl From<PseduIntentionInfoWrapper> for PseduIntentionInfoV1 {
    fn from(info_wrapper: PseduIntentionInfoWrapper) -> Self {
        Self {
            psedu_intention_common: info_wrapper.psedu_intention_common,
            psedu_intention_profs: match info_wrapper.psedu_intention_profs_wrapper {
                Ok(x) => {
                    let x: xtokens::PseduIntentionVoteWeightV1<Balance> = x.into();
                    x.into()
                }
                Err(x) => x.into(),
            },
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionVoteWeightV1ForRpc {
    /// vote weight at last update
    pub last_total_deposit_weight: String,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}

impl From<xtokens::PseduIntentionVoteWeightV1<Balance>> for PseduIntentionVoteWeightV1ForRpc {
    fn from(d1: PseduIntentionVoteWeightV1<Balance>) -> Self {
        Self {
            last_total_deposit_weight: format!("{}", d1.last_total_deposit_weight),
            last_total_deposit_weight_update: d1.last_total_deposit_weight_update,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecordCommon {
    /// name of intention
    pub id: String,
    /// total deposit
    pub balance: Balance,
    pub next_claim: BlockNumber,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecord {
    #[serde(flatten)]
    pub common: PseduNominationRecordCommon,
    /// vote weight at last update
    pub last_total_deposit_weight: u64,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecordV1 {
    #[serde(flatten)]
    pub common: PseduNominationRecordCommon,
    /// vote weight at last update
    pub last_total_deposit_weight: String,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WithdrawStatus {
    Applying,
    Signing,
    Broadcasting,
    Processing,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositInfo {
    pub time: Timestamp,
    /// txid
    pub txid: String,
    /// deposit-balance
    pub balance: Balance,
    /// token id
    pub token: String,
    /// accountid
    pub accountid: Option<AccountIdForRpc>,
    /// btc-address
    pub address: String,
    /// OP_RETURN
    pub memo: String,
    /// Confirmed height
    pub confirm: u32,
    /// Total confirmation height
    pub total_confirm: u32,
}

impl From<RecordInfo<AccountId, Balance, BlockNumber, Timestamp>> for DepositInfo {
    fn from(record: RecordInfo<AccountId, Balance, BlockNumber, Timestamp>) -> Self {
        let (confirm, total_confirm) =
            if let TxState::Confirming(confirm, total_confirm) = record.state {
                (confirm, total_confirm)
            } else {
                panic!("deposit record only has comfirm state");
            };

        let time = if let HeightOrTime::<BlockNumber, Timestamp>::Timestamp(time) =
            record.height_or_time
        {
            time
        } else {
            panic!("deposit record should be timestamp, not height");
        };

        DepositInfo {
            time,
            txid: format!("0x{:}", record.txid.to_hex::<String>()),
            balance: record.balance,
            token: String::from_utf8_lossy(&record.token).into_owned(),
            accountid: if record.who == Default::default() {
                None
            } else {
                Some(record.who.into())
            },
            address: String::from_utf8_lossy(&record.addr).into_owned(),
            memo: if record.ext.len() > 2
                && String::from_utf8_lossy(&record.token).into_owned() == "BTC"
            {
                String::from_utf8_lossy(&record.ext[2..]).into_owned()
            } else {
                String::from_utf8_lossy(&record.ext).into_owned()
            },
            confirm,
            total_confirm,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositLimit {
    pub minimal_deposit: Balance,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawInfo {
    pub height: BlockNumber,
    ///id
    pub id: u32,
    /// txid
    pub txid: String,
    /// withdraw-balance
    pub balance: Balance,
    /// token id
    pub token: String,
    /// accountid
    pub accountid: AccountIdForRpc,
    /// btc-address
    pub address: String,
    /// withdraw tx status
    pub status: Value,
    /// application status
    pub application_status: String,
    /// OP_RETURN
    pub memo: String,
}

impl From<RecordInfo<AccountId, Balance, BlockNumber, Timestamp>> for WithdrawInfo {
    fn from(record: RecordInfo<AccountId, Balance, BlockNumber, Timestamp>) -> Self {
        let height =
            if let HeightOrTime::<BlockNumber, Timestamp>::Height(height) = record.height_or_time {
                height
            } else {
                panic!("deposit record should be timestamp, not height");
            };

        let status = match record.state {
            TxState::Confirming(a, b) => json!({
                "value": "Confirming",
                "confirm": a,
                "totalConfirm": b,
            }),
            _ => json!({
                "value": record.state
            }),
        };

        WithdrawInfo {
            height,
            id: record.withdrawal_id,
            txid: if record.txid.len() > 0 {
                format!("0x{:}", record.txid.to_hex::<String>())
            } else {
                "".to_string()
            },
            balance: record.balance,
            token: String::from_utf8_lossy(&record.token).into_owned(),
            accountid: record.who.into(),
            address: String::from_utf8_lossy(&record.addr).into_owned(),
            status,
            application_status: format!(
                "{:?}",
                record
                    .application_state
                    .expect("application state must exist for withdrawal records, not None; qed")
            ),
            memo: String::from_utf8_lossy(&record.ext).into_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawTxInfo {
    /// tx
    pub tx: String,
    /// sign_status
    pub sign_status: bool,
    pub withdrawal_id_list: Vec<u32>,
    pub trustee_list: Vec<(AccountId, bool)>,
}
impl WithdrawTxInfo {
    pub fn from_bitcoin_proposal(proposal: xbitcoin::WithdrawalProposal<AccountId>) -> Self {
        let bytes = btc_serialize(&proposal.tx);
        let tx: String = bytes.to_hex();
        WithdrawTxInfo {
            tx: "0x".to_string() + &tx,
            sign_status: if proposal.sig_state == VoteResult::Finish {
                true
            } else {
                false
            },
            withdrawal_id_list: proposal.withdrawal_id_list,
            trustee_list: proposal.trustee_list,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetails {
    pub submitter: AccountIdForRpc,
    pub pair_index: TradingPairIndex,
    pub side: Side,
    pub amount: Balance,
    pub price: Balance,
    pub index: OrderIndex,
    pub order_type: OrderType,
    pub created_at: BlockNumber,
    pub status: OrderStatus,
    pub remaining: Balance,
    pub executed_indices: Vec<TradeHistoryIndex>,
    pub already_filled: Balance,
    pub last_update_at: BlockNumber,
}

impl From<OrderInfo<Runtime>> for OrderDetails {
    fn from(order: OrderInfo<Runtime>) -> Self {
        OrderDetails {
            submitter: order.submitter().into(),
            pair_index: order.pair_index(),
            side: order.side(),
            amount: order.amount(),
            price: order.price(),
            index: order.index(),
            order_type: order.order_type(),
            created_at: order.created_at(),
            status: order.status,
            remaining: order.remaining,
            executed_indices: order.executed_indices,
            already_filled: order.already_filled,
            last_update_at: order.last_update_at,
        }
    }
}

/// OrderPair info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairInfo {
    pub id: TradingPairIndex,
    pub assets: String,
    pub currency: String,
    pub precision: u32,      //价格精度
    pub unit_precision: u32, //最小单位精度
    pub online: bool,
    pub last_price: Balance,
    pub aver_price: Balance,
    pub update_height: BlockNumber,
    pub buy_one: Balance,
    pub maximum_bid: Balance,
    pub sell_one: Balance,
    pub minimum_offer: Balance,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotationsList {
    pub id: TradingPairIndex,
    pub piece: u32,
    pub sell: Vec<(Balance, Balance)>,
    pub buy: Vec<(Balance, Balance)>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfoCommon {
    /// name of intention
    pub id: String,
    /// circulation of id
    pub circulation: Balance,
    pub price: Balance,
    pub discount: u32,
    pub power: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// jackpot account
    pub jackpot_account: AccountIdForRpc,
}
