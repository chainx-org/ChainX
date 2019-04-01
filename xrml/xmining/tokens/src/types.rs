// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

use super::{Token, Trait};

/// This module only tracks the vote weight related changes.
/// All the amount related has been taken care by assets module.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct PseduIntentionVoteWeight<BlockNumber: Default> {
    pub last_total_deposit_weight: u64,
    pub last_total_deposit_weight_update: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct DepositVoteWeight<BlockNumber: Default> {
    pub last_deposit_weight: u64,
    pub last_deposit_weight_update: BlockNumber,
}

/// `PseduIntentionProfs` and `DepositRecord` is to wrap the vote weight of token,
/// sharing the vote weight calculation logic originated from staking module.
pub struct PseduIntentionProfs<'a, T: Trait> {
    pub token: &'a Token,
    pub staking: &'a mut PseduIntentionVoteWeight<T::BlockNumber>,
}

pub struct DepositRecord<'a, T: Trait> {
    pub depositor: &'a T::AccountId,
    pub token: &'a Token,
    pub staking: &'a mut DepositVoteWeight<T::BlockNumber>,
}
