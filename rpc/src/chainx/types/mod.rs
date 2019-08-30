// Copyright 2018-2019 Chainpool.

use super::*;

mod xstaking_types;
mod xtokens_types;

pub use xstaking_types::*;
pub use xtokens_types::*;

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
            name: to_string!(&asset.token()),
            token_name: to_string!(&asset.token_name()),
            chain: asset.chain(),
            precision: asset.precision(),
            desc: to_string!(&asset.desc()),
            online,
            details,
            limit_props,
        }
    }
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
            token: to_string!(&record.token),
            accountid: if record.who == Default::default() {
                None
            } else {
                Some(record.who.into())
            },
            address: to_string!(&record.addr),
            memo: if record.ext.len() > 2 && to_string!(&record.token) == "BTC" {
                to_string!(&record.ext[2..])
            } else {
                to_string!(&record.ext)
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
            token: to_string!(&record.token),
            accountid: record.who.into(),
            address: to_string!(&record.addr),
            status,
            application_status: format!(
                "{:?}",
                record
                    .application_state
                    .expect("application state must exist for withdrawal records, not None; qed")
            ),
            memo: to_string!(&record.ext),
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
