use super::*;
use codec::{Decode, Encode};
use frame_support::storage::IterableStorageDoubleMap;
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use sp_std::collections::btree_map::BTreeMap;
use xpallet_support::{RpcBalance, RpcWeightType};

/// Vote weight properties of validator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcValidatorLedger<RpcBalance, BlockNumber> {
    /// The total amount of all the nominators' vote balances.
    pub total: RpcBalance,
    /// Last calculated total vote weight of current validator.
    pub last_total_vote_weight: RpcWeightType,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_total_vote_weight_update: BlockNumber,
}

impl<Balance, BlockNumber> From<ValidatorLedger<Balance, BlockNumber>>
    for RpcValidatorLedger<RpcBalance<Balance>, BlockNumber>
{
    fn from(ledger: ValidatorLedger<Balance, BlockNumber>) -> Self {
        let last_total_vote_weight: RpcWeightType = ledger.last_total_vote_weight.into();
        let total: RpcBalance<Balance> = ledger.total.into();
        Self {
            total,
            last_total_vote_weight,
            last_total_vote_weight_update: ledger.last_total_vote_weight_update,
        }
    }
}

/// Vote weight properties of nominator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcNominatorLedger<RpcBalance, BlockNumber> {
    /// The amount of
    pub nomination: RpcBalance,
    /// Last calculated total vote weight of current nominator.
    pub last_vote_weight: RpcWeightType,
    /// Block number at which point `last_vote_weight` just updated.
    pub last_vote_weight_update: BlockNumber,
}

impl<Balance, BlockNumber> From<NominatorLedger<Balance, BlockNumber>>
    for RpcNominatorLedger<RpcBalance<Balance>, BlockNumber>
{
    fn from(ledger: NominatorLedger<Balance, BlockNumber>) -> Self {
        let nomination: RpcBalance<Balance> = ledger.nomination.into();
        let last_vote_weight: RpcWeightType = ledger.last_vote_weight.into();
        Self {
            nomination,
            last_vote_weight,
            last_vote_weight_update: ledger.last_vote_weight_update,
        }
    }
}

/// Total information about a validator.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ValidatorInfo<AccountId, RpcBalance, BlockNumber> {
    /// AccountId of this (potential) validator.
    pub account: AccountId,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub profile: ValidatorProfile<BlockNumber>,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub ledger: RpcValidatorLedger<RpcBalance, BlockNumber>,
    /// Being a validator, responsible for authoring the new blocks.
    pub is_validating: bool,
    /// How much balances the validator has bonded itself.
    pub self_bonded: RpcBalance,
    /// AccountId of the reward pot of this validator.
    pub reward_pot_account: AccountId,
    /// Balance of the reward pot account.
    pub reward_pot_balance: RpcBalance,
}

impl<T: Trait> Module<T> {
    pub fn validators_info(
    ) -> Vec<ValidatorInfo<T::AccountId, RpcBalance<T::Balance>, T::BlockNumber>> {
        Self::validator_set().map(Self::validator_info_of).collect()
    }

    pub fn validator_info_of(
        who: T::AccountId,
    ) -> ValidatorInfo<T::AccountId, RpcBalance<T::Balance>, T::BlockNumber> {
        let profile = Validators::<T>::get(&who);
        let ledger: RpcValidatorLedger<RpcBalance<T::Balance>, T::BlockNumber> =
            ValidatorLedgers::<T>::get(&who).into();
        let self_bonded: RpcBalance<T::Balance> =
            Nominations::<T>::get(&who, &who).nomination.into();
        let is_validating = T::SessionInterface::validators().contains(&who);
        let reward_pot_account = T::DetermineRewardPotAccount::reward_pot_account_for(&who);
        let reward_pot_balance: RpcBalance<T::Balance> =
            xpallet_assets::Module::<T>::pcx_free_balance(&reward_pot_account).into();
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

    pub fn staking_dividend_of(
        who: T::AccountId,
    ) -> BTreeMap<T::AccountId, RpcBalance<T::Balance>> {
        let current_block = <frame_system::Module<T>>::block_number();
        Nominations::<T>::iter_prefix(&who)
            .filter_map(|(validator, _)| {
                match Self::compute_dividend_at(&who, &validator, current_block) {
                    Ok(dividend) => Some((validator, dividend.into())),
                    Err(_) => None,
                }
            })
            .collect()
    }

    pub fn nomination_details_of(
        who: T::AccountId,
    ) -> BTreeMap<T::AccountId, RpcNominatorLedger<RpcBalance<T::Balance>, T::BlockNumber>> {
        Nominations::<T>::iter_prefix(&who)
            .map(|(validator, ledger)| (validator, ledger.into()))
            .collect()
    }
}
