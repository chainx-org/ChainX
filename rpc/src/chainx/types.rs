use std::convert::From;

use rustc_hex::ToHex;
use serde_derive::{Deserialize, Serialize};

use super::*;

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
    pub details: CodecBTreeMap<AssetType, Balance>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalAssetInfo {
    name: String,
    token_name: String,
    chain: Chain,
    precision: u16,
    desc: String,
    online: bool,
    details: CodecBTreeMap<AssetType, Balance>,
}

impl TotalAssetInfo {
    pub fn new(
        asset: Asset,
        online: bool,
        details: CodecBTreeMap<AssetType, Balance>,
    ) -> TotalAssetInfo {
        TotalAssetInfo {
            name: String::from_utf8_lossy(&asset.token()).into_owned(),
            token_name: String::from_utf8_lossy(&asset.token_name()).into_owned(),
            chain: asset.chain(),
            precision: asset.precision(),
            desc: String::from_utf8_lossy(&asset.desc()).into_owned(),
            online,
            details,
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
pub struct NominationRecord {
    pub nomination: Balance,
    pub last_vote_weight: u64,
    pub last_vote_weight_update: BlockNumber,
    pub revocations: Vec<Revocation>,
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfo {
    /// account id of intention
    pub account: AccountIdForRpc,
    /// name of intention
    pub name: String,
    /// url
    pub url: String,
    /// about
    pub about: String,
    /// is running for the validators
    pub is_active: bool,
    /// is validator
    pub is_validator: bool,
    /// is trustee
    pub is_trustee: Vec<Chain>,
    /// session key for block authoring
    pub session_key: AccountIdForRpc,
    /// how much has intention voted for itself
    pub self_vote: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// jackpot address
    pub jackpot_address: AccountIdForRpc,
    /// total nomination from all nominators
    pub total_nomination: Balance,
    /// vote weight at last update
    pub last_total_vote_weight: u64,
    /// last update time of vote weight
    pub last_total_vote_weight_update: BlockNumber,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrusteeInfo {
    chain: Chain,
    hot_entity: String,
    cold_entity: String,
}

impl TrusteeInfo {
    pub fn new(chain: Chain, hot_entity: String, cold_entity: String) -> Self {
        TrusteeInfo {
            chain,
            hot_entity,
            cold_entity,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentTrusteeSessionInfo {
    pub session_number: u32,
    pub trustee_list: Vec<AccountIdForRpc>,
    pub hot_entity: String,
    pub cold_entity: String,
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
    pub sell_one: Balance,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotationsList {
    pub id: TradingPairIndex,
    pub piece: u32,
    pub sell: Vec<(Balance, Balance)>,
    pub buy: Vec<(Balance, Balance)>,
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfo {
    /// name of intention
    pub id: String,
    /// circulation of id
    pub circulation: Balance,
    pub price: Balance,
    pub power: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// jackpot address
    pub jackpot_address: AccountIdForRpc,
    /// vote weight at last update
    pub last_total_deposit_weight: u64,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecord {
    /// name of intention
    pub id: String,
    /// total deposit
    pub balance: Balance,
    /// vote weight at last update
    pub last_total_deposit_weight: u64,
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
    pub time: u64,
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

impl From<RecordInfo<AccountId, Balance, Timestamp>> for DepositInfo {
    fn from(record: RecordInfo<AccountId, Balance, Timestamp>) -> Self {
        let (confirm, total_confirm) =
            if let TxState::Confirming(confirm, total_confirm) = record.state {
                (confirm, total_confirm)
            } else {
                panic!("deposit record only has comfirm state")
            };

        DepositInfo {
            time: record.time,
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawInfo {
    pub time: u64,
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
    /// withdraw status
    pub status: TxState,
    /// OP_RETURN
    pub memo: String,
}

impl From<RecordInfo<AccountId, Balance, Timestamp>> for WithdrawInfo {
    fn from(record: RecordInfo<AccountId, Balance, Timestamp>) -> Self {
        WithdrawInfo {
            time: record.time,
            id: record.withdrawal_id,
            txid: format!("0x{:}", record.txid.to_hex::<String>()),
            balance: record.balance,
            token: String::from_utf8_lossy(&record.token).into_owned(),
            accountid: record.who.into(),
            address: String::from_utf8_lossy(&record.addr).into_owned(),
            status: record.state,
            memo: String::from_utf8_lossy(&record.ext).into_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawTxInfo {
    /// tx
    pub tx: String,
    /// redeem_script
    pub redeem_script: String,
    /// sign_status
    pub sign_status: bool,

    pub trustee_list: Vec<(AccountId, bool)>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderPropInfo<Pair, AccountId, Amount, Price, BlockNumber>(
    AccountId,
    Pair,
    OrderDirection,
    Amount,
    Price,
    ID,
    OrderType,
    BlockNumber,
);

impl From<OrderProperty<TradingPairIndex, AccountId, Balance, Balance, BlockNumber>>
    for OrderPropInfo<TradingPairIndex, AccountIdForRpc, Balance, Balance, BlockNumber>
{
    fn from(
        prop: OrderProperty<TradingPairIndex, AccountId, Balance, Balance, BlockNumber>,
    ) -> Self {
        OrderPropInfo(
            prop.submitter().into(),
            prop.pair(),
            prop.direction(),
            prop.amount(),
            prop.price(),
            prop.index(),
            prop.order_type(),
            prop.created_at(),
        )
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInfo {
    pub props: OrderPropInfo<TradingPairIndex, AccountIdForRpc, Balance, Balance, BlockNumber>,
    pub status: OrderStatus,
    pub remaining: Balance,
    pub fill_index: Vec<ID>,
    pub already_filled: Balance,
    pub last_update_at: BlockNumber,
}

impl From<OrderDetails<Runtime>> for OrderInfo {
    fn from(order: OrderDetails<Runtime>) -> Self {
        OrderInfo {
            props: order.props.into(),
            status: order.status,
            remaining: order.remaining,
            fill_index: order.fill_index,
            already_filled: order.already_filled,
            last_update_at: order.last_update_at,
        }
    }
}
