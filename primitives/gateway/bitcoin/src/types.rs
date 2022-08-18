// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;

use chainx_primitives::ReferralId;

use light_bitcoin::keys::Address;
pub use xp_gateway_common::OpReturnAccount;

/// (hot trustee address, cold trustee address)
pub type TrusteePair = (Address, Address);

/// The bitcoin transaction type.
#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(PartialEq, Eq, Clone, RuntimeDebug, TypeInfo)]
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
#[derive(PartialEq, Eq, Clone, RuntimeDebug, TypeInfo)]
pub struct BtcDepositInfo<AccountId> {
    /// The deposit value.
    pub deposit_value: u64,
    /// The parsed op_return data.
    pub op_return: Option<(OpReturnAccount<AccountId>, Option<ReferralId>)>,
    /// The input address of deposit transaction.
    pub input_addr: Option<Address>,
}
