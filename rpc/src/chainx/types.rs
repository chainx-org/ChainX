// Copyright 2018-2019 Chainpool.

use std::collections::btree_map::BTreeMap;
use std::convert::From;
use std::iter::FromIterator;

use log::error;
use rustc_hex::ToHex;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

// chainx
use chainx_primitives::{AccountId, AccountIdForRpc, Balance, BlockNumber, Timestamp};
use chainx_runtime::Runtime;

use xassets::{Asset, AssetType, Chain};
use xrecords::{HeightOrTime, RecordInfo, TxState};
use xspot::{
    OrderIndex, OrderInfo, OrderStatus, OrderType, Side, TradeHistoryIndex, TradingPairIndex,
};

use xbitcoin::VoteResult;
use xbridge_common::{
    traits::IntoVecu8,
    types::{GenericAllSessionInfo, GenericTrusteeIntentionProps},
};
use xbridge_features::trustees::{BitcoinPublic, BitcoinTrusteeAddrInfo};

use btc_keys::DisplayLayout;
use btc_ser::serialize as btc_serialize;

use xr_primitives::generic::b58;

use super::utils::try_hex_or_str;

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
    name: String,
    token_name: String,
    chain: Chain,
    precision: u16,
    desc: String,
    online: bool,
    details: BTreeMap<AssetType, Balance>,
}

impl TotalAssetInfo {
    pub fn new(
        asset: Asset,
        online: bool,
        details: BTreeMap<AssetType, Balance>,
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

fn parse_generic_trustee_props(
    chain: Chain,
    props: &GenericTrusteeIntentionProps,
) -> Option<Value> {
    let result = match chain {
        Chain::Bitcoin => {
            let hot_public = BitcoinPublic::from_vecu8(props.0.hot_entity.as_slice());
            let cold_public = BitcoinPublic::from_vecu8(props.0.cold_entity.as_slice());
            if hot_public.is_none() || cold_public.is_none() {
                error!(
                    "parse_generic_trustee_props for bitcoin error|hot_entity:{:}|cold_entity:{:}",
                    try_hex_or_str(&props.0.hot_entity),
                    try_hex_or_str(&props.0.cold_entity)
                );
                return None;
            } else {
                let format_public = |public: &BitcoinPublic| -> Option<String> {
                    match public {
                        BitcoinPublic::Normal(_) => {
                            error!("bitcoin TrusteeIntentionProps entity should be `Compressed`, not `Normal`, something wrong in chain!|public:{:?}", public);
                            return None;
                        }
                        BitcoinPublic::Compressed(ref hash) => Some(format!("{:?}", hash)),
                    }
                };
                json!({
                    "about": String::from_utf8_lossy(&props.0.about).into_owned(),
                    "hotEntity": format_public(&hot_public.unwrap()),
                    "coldEntity": format_public(&cold_public.unwrap()),
                })
            }
        }
        // TODO when add other trustee, must add related parse here
        _ => unimplemented!("not support for other chain"),
    };
    Some(result)
}

pub fn parse_trustee_props(map: BTreeMap<Chain, GenericTrusteeIntentionProps>) -> Option<Value> {
    let map = Map::from_iter(map.into_iter().map(|(chain, generic_props)| {
        (
            format!("{:?}", chain),
            parse_generic_trustee_props(chain, &generic_props).unwrap_or(Value::Null),
        )
    }));
    Some(Value::Object(map))
}

pub fn parse_trustee_session_addr(chain: Chain, addr: &[u8]) -> Option<Value> {
    let result = match chain {
        Chain::Bitcoin => {
            let trustee_addr_info = BitcoinTrusteeAddrInfo::from_vecu8(addr);
            let trustee_addr_info = if trustee_addr_info.is_none() {
                return None;
            } else {
                trustee_addr_info.unwrap()
            };

            let address =
                String::from_utf8_lossy(&b58::to_base58(trustee_addr_info.addr.layout().to_vec()))
                    .into_owned();
            json!({
                "addr": address,
                "redeemScript": try_hex_or_str(&trustee_addr_info.redeem_script)
            })
        }
        // TODO when add other trustee, must add related parse here
        _ => unimplemented!("not support for other chain"),
    };
    Some(result)
}

pub fn parse_trustee_session_info(
    chain: Chain,
    number: u32,
    info: GenericAllSessionInfo<AccountId>,
) -> Option<Value> {
    let hot = parse_trustee_session_addr(chain, &info.hot_entity);
    let cold = parse_trustee_session_addr(chain, &info.cold_entity);
    Some(json!({
        "sessionNumber": number,
        "hotEntity": hot,
        "coldEntity": cold,
        "counts": info.counts,
        "trusteeList": info.trustees_info.into_iter().map(|(accountid, generic_props)| {
            let accountid :AccountIdForRpc= accountid.into();
            json!({
                "accountId": accountid,
                "props": parse_generic_trustee_props(chain, &generic_props),
            })
        }).collect::<Vec<_>>()
    }))
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

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfo {
    /// name of intention
    pub id: String,
    /// circulation of id
    pub circulation: Balance,
    pub price: Balance,
    pub discount: u32,
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
    /// withdraw status
    pub status: Value,
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
                "total_confirm": b,
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
