use super::*;

/// Cert info
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertInfo {
    /// name of cert
    pub name: String,
    /// when is the cert issued at
    pub issued_at: DateTime<Utc>,
    /// frozen duration of the shares cert owner holds
    pub frozen_duration: u32,
    /// remaining share of the cert
    pub remaining_shares: u32,
}

#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    pub name: String,
    pub is_native: bool,
    pub details: CodecBTreeMap<AssetType, Balance>,
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfo {
    /// name of intention
    pub name: String,
    /// activator
    pub activator: String,
    /// initial shares
    pub initial_shares: u32,
    /// url
    pub url: String,
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
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairInfo {
    pub id: OrderPairID,
    pub assets: String,
    pub currency: String,
    pub precision: u32, //价格精度
    pub used: bool,
}

#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotationsList {
    pub id: OrderPairID,
    pub piece: u32,
    pub sell: Vec<(Balance, Balance)>,
    pub buy: Vec<(Balance, Balance)>,
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct OrderList {
    pub page_size: u32,
    pub page_index: u32,
    pub page_total: u32,
    pub data: Vec<OrderT<Runtime>>,
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfo {
    /// name of intention
    pub id: Token,
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

#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecord {
    /// name of intention
    pub id: Token,
    /// total deposit
    pub balance: Balance,
    /// vote weight at last update
    pub last_total_deposit_weight: u64,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}
