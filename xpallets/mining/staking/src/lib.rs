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
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use sp_std::prelude::*;
use types::*;
use xp_staking::{CollectAssetMiningInfo, OnMinting, UnbondedIndex};

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;

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

        /// Minimum value (self_bonded, total_bonded) to be a candidate of validator election.
        pub ValidatorCandidateRequirement get(fn minimum_candidate_requirement):
            CandidateRequirement<T::Balance>;

        /// The length of a staking era in sessions.
        pub SessionsPerEra get(fn sessions_per_era) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(1000);

        /// The length of the bonding duration in blocks.
        pub BondingDuration get(fn bonding_duration) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(1000);

        /// The length of the bonding duration in blocks for intention.
        pub ValidatorBondingDuration get(fn validator_bonding_duration) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(10_000);

        /// The map from (wannabe) validator key to the profile of that validator.
        pub Validators get(fn validators):
            map hasher(twox_64_concat) T::AccountId => ValidatorProfile<T::BlockNumber>;

        /// The map from nominator key to the set of keys of all validators to nominate.
        pub Nominators get(fn nominators):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) T::AccountId
            => NominatorProfile<T::BlockNumber>;

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
        Withdrawn(AccountId, Balance),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Zero amount
        ZeroBalance,
        /// Invalid validator target.
        InvalidValidator,
        /// Stash is already bonded.
        AlreadyBonded,
        /// Controller is already paired.
        AlreadyPaired,
        /// Duplicate index.
        DuplicateIndex,
        /// Slash record index out of bounds.
        InvalidSlashIndex,
        /// Can not force validator to be chilled.
        CannotForceChilled,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
        /// Rewards for this era have already been claimed for this validator.
        AlreadyClaimed,
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
        !Validators::<T>::get(who).is_chilled
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

    fn try_fore_chilled(who: &T::AccountId) -> Result<(), Error<T>> {
        if Self::can_force_chilled() {
            return Err(Error::<T>::CannotForceChilled);
        }
        // TODO: apply_force_chilled()
        Ok(())
    }

    fn is_bonding_validator_self(nominator: &T::AccountId, nominee: &T::AccountId) -> bool {
        Self::is_validator(nominator) && *nominator == *nominee
    }
}
