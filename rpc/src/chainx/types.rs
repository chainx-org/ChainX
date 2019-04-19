// Copyright 2018-2019 Chainpool.

use std::convert::From;

use rustc_hex::ToHex;
use serde_derive::{Deserialize, Serialize};

use btc_keys::DisplayLayout;
use btc_ser::serialize as btc_serialize;

use xr_primitives::generic::b58;

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
            hot_entity: "0x".to_string() + &hot_entity,
            cold_entity: "0x".to_string() + &cold_entity,
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
    pub status: TxState,
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

        WithdrawInfo {
            height,
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
impl WithdrawTxInfo {
    pub fn new(proposal: xbitcoin::WithdrawalProposal<AccountId>, script: String) -> Self {
        let bytes = btc_serialize(&proposal.tx);
        let tx: String = bytes.to_hex();
        WithdrawTxInfo {
            tx: "0x".to_string() + &tx,
            redeem_script: "0x".to_string() + &script,
            sign_status: if proposal.sig_state == VoteResult::Finish {
                true
            } else {
                false
            },
            trustee_list: proposal.trustee_list,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetails {
    pub submitter: AccountIdForRpc,
    pub pair_index: TradingPairIndex,
    pub direction: OrderDirection,
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
            direction: order.direction(),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitcoinAddrEntity {
    pub address: String,
    pub redeem_script: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitcoinTrusteeInfo {
    account_id: AccountIdForRpc,
    hot_pubkey: String,
    cold_pubkey: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitcoinMultiSigCount {
    required: u32,
    total: u32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockBitcoinTrustee {
    /// accountid, hot pubkey, cold pubkey
    pub trustee_info: Vec<BitcoinTrusteeInfo>,
    pub counts: BitcoinMultiSigCount,
    pub hot_entity: BitcoinAddrEntity,
    pub cold_entity: BitcoinAddrEntity,
}

impl
    From<(
        Vec<(AccountId, (Vec<u8>, Vec<u8>))>,
        (u32, u32),
        BtcTrusteeAddrInfo,
        BtcTrusteeAddrInfo,
    )> for MockBitcoinTrustee
{
    fn from(
        info: (
            Vec<(AccountId, (Vec<u8>, Vec<u8>))>,
            (u32, u32),
            BtcTrusteeAddrInfo,
            BtcTrusteeAddrInfo,
        ),
    ) -> Self {
        let trustee_info: Vec<BitcoinTrusteeInfo> = info
            .0
            .into_iter()
            .map(|info| {
                let hot: String = (info.1).0.to_hex();
                let cold: String = (info.1).1.to_hex();
                BitcoinTrusteeInfo {
                    account_id: info.0.into(),
                    hot_pubkey: "0x".to_string() + &hot,
                    cold_pubkey: "0x".to_string() + &cold,
                }
            })
            .collect();

        let hot_script: String = info.2.redeem_script.to_hex();
        let cold_script: String = info.3.redeem_script.to_hex();
        MockBitcoinTrustee {
            trustee_info,
            counts: BitcoinMultiSigCount {
                required: (info.1).0,
                total: (info.1).1,
            },
            hot_entity: BitcoinAddrEntity {
                address: String::from_utf8_lossy(&b58::to_base58(info.2.addr.layout().to_vec()))
                    .into_owned(),
                redeem_script: "0x".to_string() + &hot_script,
            },
            cold_entity: BitcoinAddrEntity {
                address: String::from_utf8_lossy(&b58::to_base58(info.3.addr.layout().to_vec()))
                    .into_owned(),
                redeem_script: "0x".to_string() + &cold_script,
            },
        }
    }
}
