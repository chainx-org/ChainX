use rstd::prelude::Vec;
use {AddrStr, Application, Memo, Token};

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum DepositState {
    Confirming(u32, u32),
    Unknown,
}

impl Default for DepositState {
    fn default() -> Self {
        DepositState::Unknown
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct DepositLog<AccountId, Balance, Moment> {
    pub depositor: AccountId,
    pub token: Token,
    pub balance: Balance,
    pub txid: Vec<u8>,
    pub addr: AddrStr,
    pub ext: Memo,
    pub time: Moment,
    pub state: DepositState,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum WithdrawalState {
    Applying,
    Signing,
    Broadcast,
    Confirming(u32, u32),
    Unknown,
}

impl Default for WithdrawalState {
    fn default() -> Self {
        WithdrawalState::Unknown
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct WithdrawalLog<AccountId, Balance, Moment> {
    #[cfg_attr(feature = "std", serde(flatten))]
    application: Application<AccountId, Balance, Moment>,
    state: WithdrawalState,
}

impl<AccountId, Balance, Moment> WithdrawalLog<AccountId, Balance, Moment> {
    pub fn new(appl: Application<AccountId, Balance, Moment>, state: WithdrawalState) -> Self {
        WithdrawalLog {
            application: appl,
            state,
        }
    }
}
