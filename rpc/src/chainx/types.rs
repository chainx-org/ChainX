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
    pub details: CodecBTreeMap<AssetTypeWrapper, Balance>,
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

#[derive(Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum AssetTypeWrapper {
    Free,
    ReservedStaking,
    ReservedStakingRevocation,
    ReservedWithdrawal,
    ReservedDexSpot,
    ReservedDexFuture,
}

impl Default for AssetTypeWrapper {
    fn default() -> Self {
        AssetTypeWrapper::Free
    }
}

impl AssetTypeWrapper {
    pub fn new(type_: AssetType) -> AssetTypeWrapper {
        match type_ {
            AssetType::Free => AssetTypeWrapper::Free,
            AssetType::ReservedStaking => AssetTypeWrapper::ReservedStaking,
            AssetType::ReservedStakingRevocation => AssetTypeWrapper::ReservedStakingRevocation,
            AssetType::ReservedWithdrawal => AssetTypeWrapper::ReservedWithdrawal,
            AssetType::ReservedDexSpot => AssetTypeWrapper::ReservedDexSpot,
            AssetType::ReservedDexFuture => AssetTypeWrapper::ReservedDexFuture,
        }
    }
}

#[derive(Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ChainWrapper {
    ChainX,
    Bitcoin,
    Ethereum,
}
impl ChainWrapper {
    pub fn new(type_: Chain) -> ChainWrapper {
        match type_ {
            Chain::ChainX => ChainWrapper::ChainX,
            Chain::Bitcoin => ChainWrapper::Bitcoin,
            Chain::Ethereum => ChainWrapper::Ethereum,
        }
    }
}

impl Default for ChainWrapper {
    fn default() -> Self {
        ChainWrapper::ChainX
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
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
