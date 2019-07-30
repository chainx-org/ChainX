// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

use super::{Token, Trait};
use primitives::traits::As;
use xstaking::{VoteWeight, VoteWeightBase};
use xsupport::trace;

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

impl<'a, T: Trait> VoteWeightBase<T::BlockNumber> for PseduIntentionProfs<'a, T> {
    fn amount(&self) -> u64 {
        xassets::Module::<T>::all_type_total_asset_balance(&self.token).as_()
    }

    fn last_acum_weight(&self) -> u64 {
        self.staking.last_total_deposit_weight
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.staking.last_total_deposit_weight_update.as_()
    }

    fn set_amount(&mut self, _: u64) {}

    fn set_last_acum_weight(&mut self, latest_deposit_weight: u64) {
        trace!(
            target: "tokens",
            "[set_last_acum_weight] [psudu_intention] amount: {:?}, last_acum_weight: {:?}, last_acum_weight_update: {:?}, current_block: {:?}, latest_deposit_weight: {:?}",
            self.amount(),
            self.last_acum_weight(),
            self.last_acum_weight_update(),
            <system::Module<T>>::block_number(),
            latest_deposit_weight
        );
        self.staking.last_total_deposit_weight = latest_deposit_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.staking.last_total_deposit_weight_update = current_block;
    }
}

impl<'a, T: Trait> VoteWeight<T::BlockNumber> for PseduIntentionProfs<'a, T> {}

impl<'a, T: Trait> VoteWeightBase<T::BlockNumber> for DepositRecord<'a, T> {
    fn amount(&self) -> u64 {
        xassets::Module::<T>::all_type_asset_balance(&self.depositor, &self.token).as_()
    }

    fn last_acum_weight(&self) -> u64 {
        self.staking.last_deposit_weight
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.staking.last_deposit_weight_update.as_()
    }

    fn set_amount(&mut self, _: u64) {}

    fn set_last_acum_weight(&mut self, latest_deposit_weight: u64) {
        trace!(
            target: "tokens",
            "[set_last_acum_weight] [depositor] amount: {:?}, last_acum_weight: {:?}, last_acum_weight_update: {:?}, current_block: {:?} => latest_deposit_weight: {:?}",
            self.amount(),
            self.last_acum_weight(),
            self.last_acum_weight_update(),
            <system::Module<T>>::block_number(),
            latest_deposit_weight
        );
        self.staking.last_deposit_weight = latest_deposit_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.staking.last_deposit_weight_update = current_block;
    }
}

impl<'a, T: Trait> VoteWeight<T::BlockNumber> for DepositRecord<'a, T> {}
