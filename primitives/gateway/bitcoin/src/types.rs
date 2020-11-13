// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;

use chainx_primitives::ReferralId;

use light_bitcoin::keys::Address;

///
#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub enum MetaTxType<AccountId> {
    Withdrawal,
    Deposit(DepositInfo<AccountId>),
    HotAndCold,
    TrusteeTransition,
    Irrelevance,
}

impl<AccountId> From<MetaTxType<AccountId>> for BtcTxType {
    fn from(meta_tx_type: MetaTxType<AccountId>) -> Self {
        match meta_tx_type {
            MetaTxType::Withdrawal => BtcTxType::Withdrawal,
            MetaTxType::Deposit(_) => BtcTxType::Deposit,
            MetaTxType::HotAndCold => BtcTxType::HotAndCold,
            MetaTxType::TrusteeTransition => BtcTxType::TrusteeTransition,
            MetaTxType::Irrelevance => BtcTxType::Irrelevance,
        }
    }
}

///
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct DepositInfo<AccountId> {
    ///
    pub deposit_value: u64,
    ///
    pub op_return: Option<(AccountId, Option<ReferralId>)>,
    ///
    pub input_addr: Option<Address>,
}

///
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
