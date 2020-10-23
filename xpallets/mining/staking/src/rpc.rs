// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use frame_support::storage::{IterableStorageDoubleMap, StorageDoubleMap, StorageMap};
use sp_runtime::RuntimeDebug;

use xp_mining_common::RewardPotAccountFor;

use crate::{
    types::*, BalanceOf, LastRebondOf, Module, Nominations, SessionInterface, Trait,
    ValidatorLedgers, Validators,
};

/// Total information about a validator.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ValidatorInfo<AccountId, Balance, VoteWeight, BlockNumber> {
    /// AccountId of this (potential) validator.
    pub account: AccountId,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub profile: ValidatorProfile<BlockNumber>,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub ledger: ValidatorLedger<Balance, VoteWeight, BlockNumber>,
    /// Being a validator, responsible for authoring the new blocks.
    pub is_validating: bool,
    /// How much balances the validator has bonded itself.
    pub self_bonded: Balance,
    /// AccountId of the reward pot of this validator.
    pub reward_pot_account: AccountId,
    /// Balance of the reward pot account.
    pub reward_pot_balance: Balance,
}

/// Profile of staking nominator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct NominatorInfo<BlockNumber> {
    /// Block number of last `rebond` operation.
    pub last_rebond: Option<BlockNumber>,
}

impl<T: Trait> Module<T> {
    pub fn validators_info(
    ) -> Vec<ValidatorInfo<T::AccountId, BalanceOf<T>, VoteWeight, T::BlockNumber>> {
        Self::validator_set().map(Self::validator_info_of).collect()
    }

    pub fn validator_info_of(
        who: T::AccountId,
    ) -> ValidatorInfo<T::AccountId, BalanceOf<T>, VoteWeight, T::BlockNumber> {
        let profile = Validators::<T>::get(&who);
        let ledger: ValidatorLedger<BalanceOf<T>, VoteWeight, T::BlockNumber> =
            ValidatorLedgers::<T>::get(&who);
        let self_bonded: BalanceOf<T> = Nominations::<T>::get(&who, &who).nomination;
        let is_validating = T::SessionInterface::validators().contains(&who);
        let reward_pot_account = T::DetermineRewardPotAccount::reward_pot_account_for(&who);
        let reward_pot_balance: BalanceOf<T> = Self::free_balance(&reward_pot_account);
        ValidatorInfo {
            account: who,
            profile,
            ledger,
            is_validating,
            self_bonded,
            reward_pot_account,
            reward_pot_balance,
        }
    }

    pub fn staking_dividend_of(who: T::AccountId) -> BTreeMap<T::AccountId, BalanceOf<T>> {
        let current_block = <frame_system::Module<T>>::block_number();
        Nominations::<T>::iter_prefix(&who)
            .filter_map(|(validator, _)| {
                match Self::compute_dividend_at(&who, &validator, current_block) {
                    Ok(dividend) => Some((validator, dividend)),
                    Err(_) => None,
                }
            })
            .collect()
    }

    pub fn nomination_details_of(
        who: T::AccountId,
    ) -> BTreeMap<T::AccountId, NominatorLedger<BalanceOf<T>, VoteWeight, T::BlockNumber>> {
        Nominations::<T>::iter_prefix(&who)
            .map(|(validator, ledger)| (validator, ledger))
            .collect()
    }

    pub fn nominator_info_of(who: T::AccountId) -> NominatorInfo<T::BlockNumber> {
        let last_rebond = LastRebondOf::<T>::get(&who);
        NominatorInfo { last_rebond }
    }
}
