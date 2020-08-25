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

/// Type for noting when the unbonded fund can be withdrawn.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcUnbonded<RpcBalance, BlockNumber> {
    /// Amount of funds to be unlocked.
    pub value: RpcBalance,
    /// Block number at which point it'll be unlocked.
    pub locked_until: BlockNumber,
}

impl<Balance, BlockNumber> From<Unbonded<Balance, BlockNumber>>
    for RpcUnbonded<RpcBalance<Balance>, BlockNumber>
{
    fn from(unbonded: Unbonded<Balance, BlockNumber>) -> Self {
        let value: RpcBalance<Balance> = unbonded.value.into();
        Self {
            value,
            locked_until: unbonded.locked_until,
        }
    }
}

/// Profile of staking nominator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct NominatorInfo<RpcBalance, BlockNumber> {
    /// Block number of last `rebond` operation.
    pub last_rebond: Option<BlockNumber>,
    /// Total unbonded entries.
    pub unbonded_chunks: Vec<RpcUnbonded<RpcBalance, BlockNumber>>,
}

impl<T: Trait> Module<T> {
    pub fn validators_info(
    ) -> Vec<ValidatorInfo<T::AccountId, RpcBalance<BalanceOf<T>>, T::BlockNumber>> {
        Self::validator_set().map(Self::validator_info_of).collect()
    }

    pub fn validator_info_of(
        who: T::AccountId,
    ) -> ValidatorInfo<T::AccountId, RpcBalance<BalanceOf<T>>, T::BlockNumber> {
        let profile = Validators::<T>::get(&who);
        let ledger: RpcValidatorLedger<RpcBalance<BalanceOf<T>>, T::BlockNumber> =
            ValidatorLedgers::<T>::get(&who).into();
        let self_bonded: RpcBalance<BalanceOf<T>> =
            Nominations::<T>::get(&who, &who).nomination.into();
        let is_validating = T::SessionInterface::validators().contains(&who);
        let reward_pot_account = T::DetermineRewardPotAccount::reward_pot_account_for(&who);
        let reward_pot_balance: RpcBalance<BalanceOf<T>> =
            Self::free_balance(&reward_pot_account).into();
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
    ) -> BTreeMap<T::AccountId, RpcBalance<BalanceOf<T>>> {
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
    ) -> BTreeMap<T::AccountId, RpcNominatorLedger<RpcBalance<BalanceOf<T>>, T::BlockNumber>> {
        Nominations::<T>::iter_prefix(&who)
            .map(|(validator, ledger)| (validator, ledger.into()))
            .collect()
    }

    pub fn nominator_info_of(
        who: T::AccountId,
    ) -> NominatorInfo<RpcBalance<BalanceOf<T>>, T::BlockNumber> {
        let nominator_profile = Nominators::<T>::get(&who);
        NominatorInfo {
            last_rebond: nominator_profile.last_rebond,
            unbonded_chunks: nominator_profile
                .unbonded_chunks
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}
