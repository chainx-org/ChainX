use super::*;

// utils
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PageData<T> {
    pub page_total: u32,
    pub page_index: u32,
    pub page_size: u32,
    pub data: Vec<T>,
}

/// Cert info
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertInfo {
    /// name of cert
    pub name: String,
    /// when is the cert issued at
    pub issued_at: Timestamp,
    /// frozen duration of the shares cert owner holds
    pub frozen_duration: u32,
    /// remaining share of the cert
    pub remaining_shares: u32,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    pub name: String,
    pub is_native: bool,
    pub details: CodecBTreeMap<AssetType, Balance>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalAssetInfo {
    name: String,
    token_name: String,
    is_native: bool,
    chain: Chain,
    precision: u16,
    desc: String,
    trustee_addr: String,
    details: CodecBTreeMap<AssetType, Balance>,
}

impl TotalAssetInfo {
    pub fn new(
        asset: Asset,
        trustee: String,
        details: CodecBTreeMap<AssetType, Balance>,
    ) -> TotalAssetInfo {
        TotalAssetInfo {
            name: String::from_utf8_lossy(&asset.token()).into_owned(),
            token_name: String::from_utf8_lossy(&asset.token_name()).into_owned(),
            is_native: asset.chain() == Chain::ChainX,
            chain: asset.chain(),
            precision: asset.precision(),
            desc: String::from_utf8_lossy(&asset.desc()).into_owned(),
            trustee_addr: trustee,
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
    /// activator
    pub activator: String,
    /// initial shares
    pub initial_shares: u32,
    /// when is the intention registered
    pub registered_at: Timestamp,
    /// url
    pub url: String,
    /// about
    pub about: String,
    /// is running for the validators
    pub is_active: bool,
    /// is validator
    pub is_validator: bool,
    /// how much has intention voted for itself
    pub self_vote: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// total nomination from all nominators
    pub total_nomination: Balance,
    /// vote weight at last update
    pub last_total_vote_weight: u64,
    /// last update time of vote weight
    pub last_total_vote_weight_update: BlockNumber,
}

/// OrderPair info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairInfo {
    pub id: OrderPairID,
    pub assets: String,
    pub currency: String,
    pub precision: u32, //价格精度
    pub unit_precision: u32,//最小单位精度
    pub used: bool,
    pub last_price: Balance,
    pub aver_price: Balance,
    pub update_height: BlockNumber,
    pub buy_one:Balance,
    pub sell_one:Balance,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotationsList {
    pub id: OrderPairID,
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
    /// jackpot
    pub jackpot: Balance,
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

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
//#[serde(rename_all = "camelCase")]
pub enum WithdrawalState {
    Applying,
    Signing,
    Unknown,
}

impl Default for WithdrawalState {
    fn default() -> Self {
        WithdrawalState::Applying
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationWrapper {
    id: u32,
    applicant: AccountId,
    token: String,
    balance: Balance,
    addr: String,
    ext: String,
    time: Timestamp,
    state: WithdrawalState,
}

impl ApplicationWrapper {
    pub fn new(
        appl: Application<AccountId, Balance, Timestamp>,
        state: WithdrawalState,
    ) -> ApplicationWrapper {
        ApplicationWrapper {
            id: appl.id(),
            applicant: appl.applicant(),
            token: String::from_utf8_lossy(&appl.token()).into_owned(),
            balance: appl.balance(),
            addr: String::from_utf8_lossy(&appl.addr()).into_owned(),
            ext: String::from_utf8_lossy(&appl.ext()).into_owned(),
            time: appl.time(),
            state,
        }
    }
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
    /// OP_RETURN
    pub remarks: String,
}
