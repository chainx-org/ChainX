// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;

use chainx_primitives::ReferralId;

use light_bitcoin::keys::MultiAddress as Address;

/// (hot trustee address, cold trustee address)
pub type TrusteePair = (Address, Address);

/// The bitcoin transaction type.
#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum BtcTxType {
    Withdrawal,
    Deposit,
    HotAndCold,
    TrusteeTransition,
    Irrelevance,
}

#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RequestMetaType {
    Issue,
    Redeem,
    Irrelevance,
}

#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RequestType {
    Issue(u128),
    Redeem(u128),
    Irrelevance,
}

#[doc(hidden)]
impl RequestType {
    pub fn ref_into(&self) -> RequestMetaType {
        match self {
            Self::Issue(_) => RequestMetaType::Issue,
            Self::Redeem(_) => RequestMetaType::Redeem,
            Self::Irrelevance => RequestMetaType::Irrelevance,
        }
    }
}

impl Default for RequestType {
    fn default() -> Self {
        Self::Irrelevance
    }
}

#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RequestInfo<AccountId> {
    pub requester: AccountId,
    pub requester_addr: Option<Address>,
    pub vault_addr: Option<Address>,
    pub amount: u64,
    pub request_id: u128,
    pub request_type: RequestMetaType,
}

impl Default for BtcTxType {
    fn default() -> Self {
        BtcTxType::Irrelevance
    }
}

/// The transaction type with deposit info.
#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub enum BtcTxMetaType<AccountId> {
    Withdrawal,
    Deposit(BtcDepositInfo<AccountId>),
    HotAndCold,
    TrusteeTransition,
    Irrelevance,
}

impl<AccountId> BtcTxMetaType<AccountId> {
    /// Convert the MetaTxType as BtcTxType.
    pub fn ref_into(&self) -> BtcTxType {
        match self {
            BtcTxMetaType::Withdrawal => BtcTxType::Withdrawal,
            BtcTxMetaType::Deposit(_) => BtcTxType::Deposit,
            BtcTxMetaType::HotAndCold => BtcTxType::HotAndCold,
            BtcTxMetaType::TrusteeTransition => BtcTxType::TrusteeTransition,
            BtcTxMetaType::Irrelevance => BtcTxType::Irrelevance,
        }
    }
}

/// The info of deposit transaction.
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct BtcDepositInfo<AccountId> {
    /// The deposit value.
    pub deposit_value: u64,
    /// The parsed op_return data.
    pub op_return: Option<(AccountId, Option<ReferralId>)>,
    /// The input address of deposit transaction.
    pub input_addr: Option<Address>,
}
