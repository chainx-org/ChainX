//! # Staking Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types;

use chainx_primitives::AssetId;
use chainx_primitives::Memo;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    storage::IterableStorageMap,
    traits::Get,
    weights::{
        DispatchInfo, GetDispatchInfo, Pays, PostDispatchInfo, Weight, WeightToFeeCoefficient,
        WeightToFeePolynomial,
    },
};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{
        Convert, DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SaturatedConversion, Saturating,
        SignedExtension, UniqueSaturatedFrom, UniqueSaturatedInto, Zero,
    },
    FixedI128, FixedPointNumber, FixedPointOperand,
};
use sp_std::prelude::*;
use types::*;
use xp_staking::{CollectAssetMiningInfo, OnMinting, UnbondedIndex};

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const DEFAULT_MAXIMUM_VALIDATOR_COUNT: u32 = 100;
const DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE: u32 = 10;

pub trait Trait: frame_system::Trait + xpallet_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    ///
    type CollectAssetMiningInfo: CollectAssetMiningInfo;
    ///
    type OnMinting: OnMinting<AssetId, Self::Balance>;
}

decl_storage! {
    trait Store for Module<T: Trait> as XStaking {
        /// The ideal number of staking participants.
        pub ValidatorCount get(fn validator_count) config(): u32;

        /// Minimum number of staking participants before emergency conditions are imposed.
        pub MinimumValidatorCount get(fn minimum_validator_count) config():
            u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;

        /// Maximum number of staking participants before emergency conditions are imposed.
        pub MaximumValidatorCount get(fn maximum_validator_count) config():
            u32 = DEFAULT_MAXIMUM_VALIDATOR_COUNT;

        /// Minimum value (self_bonded, total_bonded) to be a candidate of validator election.
        pub ValidatorCandidateRequirement get(fn minimum_candidate_requirement):
            CandidateRequirement<T::Balance>;

        /// The length of a session in blocks.
        pub BlocksPerSession get(fn blocks_per_session) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(50);

        /// The length of a staking era in sessions.
        pub SessionsPerEra get(fn sessions_per_era) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(12);

        /// The length of the bonding duration in blocks.
        pub BondingDuration get(fn bonding_duration) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(50 * 12 * 24 * 3);

        /// The length of the bonding duration in blocks for validator.
        pub ValidatorBondingDuration get(fn validator_bonding_duration) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(50 * 12 * 24 * 3 * 10);

        /// Maximum number of on-going unbonded chunk.
        pub MaximumUnbondedChunkSize get(fn maximum_unbonded_chunk_size) config():
            u32 = DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE;

        /// Maximum value of total_bonded/self_bonded.
        pub UpperBoundFactorOfAcceptableVotes get(fn upper_bound_factor) config():
            u32 = 10u32;

        /// (Treasury, Staking)
        pub GlobalDistributionRatio get(fn globaldistribution_ratio): (u32, u32) = (1u32, 1u32);

        /// (Staker, External Miners)
        pub DistributionRatio get(fn distribution_ratio): (u32, u32) = (1u32, 1u32);

        /// The map from (wannabe) validator key to the profile of that validator.
        pub Validators get(fn validators):
            map hasher(twox_64_concat) T::AccountId => ValidatorProfile<T::BlockNumber>;

        /// The map from nominator key to the set of keys of all validators to nominate.
        pub Nominators get(fn nominators):
            map hasher(twox_64_concat) T::AccountId => NominatorProfile<T::Balance, T::BlockNumber>;

        /// The map from validator key to the vote weight ledger of that validator.
        pub ValidatorLedgers get(fn validator_ledgers):
            map hasher(twox_64_concat) T::AccountId => ValidatorLedger<T::Balance, T::BlockNumber>;

        /// The map from nominator to the vote weight ledger of all nominees.
        pub Nominations get(fn nominations):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) T::AccountId
            => NominatorLedger<T::Balance, T::BlockNumber>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as xpallet_assets::Trait>::Balance,
    {
        /// The staker has been rewarded by this amount. `AccountId` is the stash account.
        Reward(AccountId, Balance),
        /// One validator (and its nominators) has been slashed by the given amount.
        Slash(AccountId, Balance),
        ///
        /// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
        /// it will not be emitted for staking rewards when they are added to stake.
        Bonded(AccountId, Balance),
        /// An account has unbonded this amount.
        Unbonded(AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue.
        WithdrawUnbonded(AccountId, Balance),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Zero amount
        ZeroBalance,
        /// Invalid validator target.
        InvalidValidator,
        /// Can not force validator to be chilled.
        InsufficientValidators,
        /// Free balance can not cover this bond operation.
        InsufficientBalance,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not schedule more unbond chunks.
        NoMoreUnbondChunks,
        /// Validators can not accept more votes from other voters.
        NoMoreAcceptableVotes,
        /// Can not rebond due to the restriction of rebond frequency limit.
        RebondNotAllowed,
        /// The call is not allowed at the given time due to restrictions of election period.
        CallNotAllowed,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn on_finalize() {
        }

        /// Nominates the `target` with `value` of the origin account's balance locked.
        #[weight = 10]
        fn bond(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::InvalidValidator);
            ensure!(value <= Self::free_balance_of(&sender), Error::<T>::InsufficientBalance);
            if !Self::is_validator_self_bonding(&sender, &target) {
                Self::check_validator_acceptable_votes_limit(&sender, value)?;
            }

            Self::apply_bond(&sender, &target, value);
        }

        /// Switchs the nomination of `value` from one validator to another.
        #[weight = 10]
        fn rebond(origin, from: T::AccountId, to: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
        }

        ///
        #[weight = 10]
        fn unbond(origin, target: T::AccountId, memo: Memo) {
            let sender = ensure_signed(origin)?;
        }

        /// Frees up the unbonded balances that are due.
        #[weight = 10]
        fn withdraw_unbonded(origin, target: T::AccountId, unbonded_index: UnbondedIndex) {
            let sender = ensure_signed(origin)?;
        }

        /// Claims the staking reward given the `target` validator.
        #[weight = 10]
        fn claim(origin, target: T::AccountId) {
            let sender = ensure_signed(origin)?;
        }

        /// Declare the desire to validate for the origin account.
        #[weight = 10]
        fn validate(origin) {
            let sender = ensure_signed(origin)?;
        }

        /// Declare no desire to validate for the origin account.
        #[weight = 10]
        fn chill(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
            memo.check_validity()?;
            for validator in Validators::<T>::iter(){}
        }

        /// TODO: figure out whether this should be kept.
        #[weight = 10]
        fn register(origin) {
            let sender = ensure_signed(origin)?;
        }
    }
}

impl<T: Trait> Module<T> {
    #[inline]
    pub fn is_validator(who: &T::AccountId) -> bool {
        Validators::<T>::contains_key(who)
    }

    #[inline]
    pub fn is_chilled(who: &T::AccountId) -> bool {
        Validators::<T>::get(who).is_chilled
    }

    #[inline]
    pub fn is_active(who: &T::AccountId) -> bool {
        !Self::is_chilled(who)
    }

    #[inline]
    fn unbonded_chunk_of(nominator: &T::AccountId) -> Vec<Unbonded<T::Balance, T::BlockNumber>> {
        Nominators::<T>::get(nominator).unbonded
    }

    #[inline]
    fn last_rebond_of(nominator: &T::AccountId) -> Option<T::BlockNumber> {
        Nominators::<T>::get(nominator).last_rebond
    }

    #[inline]
    fn free_balance_of(who: &T::AccountId) -> T::Balance {
        <xpallet_assets::Module<T>>::pcx_free_balance(who)
    }

    fn is_validator_self_bonding(nominator: &T::AccountId, nominee: &T::AccountId) -> bool {
        Self::is_validator(nominator) && *nominator == *nominee
    }

    pub fn validator_set() -> Vec<T::AccountId> {
        Validators::<T>::iter()
            .map(|(v, _)| v)
            .filter(Self::is_active)
            .collect()
    }

    fn can_force_chilled() -> bool {
        // TODO: optimize using try_for_each?
        let active = Validators::<T>::iter()
            .map(|(v, _)| v)
            .filter(Self::is_active)
            .collect::<Vec<_>>();
        active.len() > Self::minimum_validator_count() as usize
    }

    fn try_force_chilled(who: &T::AccountId) -> Result<(), Error<T>> {
        if !Self::can_force_chilled() {
            return Err(Error::<T>::InsufficientValidators);
        }
        // TODO: apply_force_chilled()
        Ok(())
    }

    fn total_votes_of(validator: &T::AccountId) -> T::Balance {
        ValidatorLedgers::<T>::get(validator).total
    }

    fn validator_self_bonded(validator: &T::AccountId) -> T::Balance {
        Nominations::<T>::get(validator, validator).value
    }

    fn acceptable_votes_limit_of(validator: &T::AccountId) -> T::Balance {
        Self::validator_self_bonded(validator) * T::Balance::from(Self::upper_bound_factor())
    }

    fn check_validator_acceptable_votes_limit(
        validator: &T::AccountId,
        value: T::Balance,
    ) -> Result<(), Error<T>> {
        let cur_total = Self::total_votes_of(validator);
        let upper_limit = Self::acceptable_votes_limit_of(validator);
        if cur_total + value <= upper_limit {
            Ok(())
        } else {
            Err(Error::<T>::NoMoreAcceptableVotes)
        }
    }

    fn apply_bond(nominator: &T::AccountId, nominee: &T::AccountId, value: T::Balance) {}
}
