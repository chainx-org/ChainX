use rstd::prelude::Vec;
use {AddrStr, Memo, Token};

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum TxState {
    NotApplying,
    Applying,
    Signing,
    Broadcasting,
    Processing,
    Confirming(u32, u32),
    Confirmed,
    Unknown,
}
impl Default for TxState {
    fn default() -> Self {
        TxState::NotApplying
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RecordInfo<AccountId, Balance, Moment> {
    pub who: AccountId,
    pub token: Token,
    pub balance: Balance,
    // txhash
    pub txid: Vec<u8>,
    /// withdrawal addr or deposit from which addr
    pub addr: AddrStr,
    /// memo or ext info
    pub ext: Memo,
    /// tx time
    pub time: Moment,
    /// only for withdrawal, mark which id for application
    pub withdrawal_id: u32, // only for withdrawal
    /// tx state
    pub state: TxState,
}
