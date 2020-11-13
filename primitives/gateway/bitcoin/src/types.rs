// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;

use chainx_primitives::ReferralId;

use light_bitcoin::keys::Address;

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

impl Default for BtcTxType {
    fn default() -> Self {
        BtcTxType::Irrelevance
    }
}

/// The transaction type with deposit info.
#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub enum MetaTxType<AccountId> {
    Withdrawal,
    Deposit(DepositInfo<AccountId>),
    HotAndCold,
    TrusteeTransition,
    Irrelevance,
}

impl<AccountId> MetaTxType<AccountId> {
    /// Convert the MetaTxType as BtcTxType.
    pub fn ref_into(&self) -> BtcTxType {
        match self {
            MetaTxType::Withdrawal => BtcTxType::Withdrawal,
            MetaTxType::Deposit(_) => BtcTxType::Deposit,
            MetaTxType::HotAndCold => BtcTxType::HotAndCold,
            MetaTxType::TrusteeTransition => BtcTxType::TrusteeTransition,
            MetaTxType::Irrelevance => BtcTxType::Irrelevance,
        }
    }
}

/// The info of deposit transaction.
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct DepositInfo<AccountId> {
    /// The deposit value.
    pub deposit_value: u64,
    /// The parsed op_return data.
    pub op_return: Option<(AccountId, Option<ReferralId>)>,
    /// The input address of deposit transaction.
    pub input_addr: Option<Address>,
}
