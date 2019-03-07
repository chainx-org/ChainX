use super::*;
use serde_derive::{Deserialize, Serialize};

// utils
#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revocation {
    pub block_numer: BlockNumber,
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

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfo {
    /// account id of intention
    pub account: AccountId,
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
    pub is_trustee: bool,
    /// session key for block authoring
    pub session_key: AccountId,
    /// how much has intention voted for itself
    pub self_vote: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// jackpot address
    pub jackpot_address: H256,
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
    pub jackpot_address: H256,
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

//#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
////#[serde(rename_all = "camelCase")]
//pub enum WithdrawalState {
//    Applying,
//    Signing,
//    Unknown,
//}

//impl Default for WithdrawalState {
//    fn default() -> Self {
//        WithdrawalState::Applying
//    }
//}
//
//#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
//#[serde(rename_all = "camelCase")]
//pub struct ApplicationWrapper {
//    id: u32,
//    applicant: AccountId,
//    token: String,
//    balance: Balance,
//    addr: String,
//    ext: String,
//    time: Timestamp,
//    state: WithdrawalState,
//}
//
//impl ApplicationWrapper {
//    pub fn new(
//        appl: Application<AccountId, Balance, Timestamp>,
//        state: WithdrawalState,
//    ) -> ApplicationWrapper {
//        ApplicationWrapper {
//            id: appl.id(),
//            applicant: appl.applicant(),
//            token: String::from_utf8_lossy(&appl.token()).into_owned(),
//            balance: appl.balance(),
//            addr: String::from_utf8_lossy(&appl.addr()).into_owned(),
//            ext: String::from_utf8_lossy(&appl.ext()).into_owned(),
//            time: appl.time(),
//            state,
//        }
//    }
//}
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
    pub time: u32,
    /// txid
    pub txid: String,
    /// Confirmed height
    pub confirm: u32,
    /// Total confirmation height
    pub total_confirm: u32,
    /// btc-address
    pub address: String,
    /// deposit-balance
    pub balance: Balance,
    /// token id
    pub token: String,
    /// accountid
    pub accountid: Option<AccountId>,
    /// OP_RETURN
    pub memo: String,
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
    pub accountid: AccountId,
    /// btc-address
    pub address: String,
    /// withdraw status
    pub status: WithdrawStatus,
    /// OP_RETURN
    pub memo: String,
}

impl WithdrawInfo {
    pub fn new(
        time: u64,
        id: u32,
        txid: String,
        balance: Balance,
        token: String,
        accountid: AccountId,
        address: String,
        status: WithdrawStatus,
        memo: String,
    ) -> Self {
        WithdrawInfo {
            time,
            id,
            txid,
            balance,
            token,
            accountid,
            address,
            status,
            memo,
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
}
