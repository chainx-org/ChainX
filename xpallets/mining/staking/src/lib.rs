// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! # Staking Pallet
//!
//! ## Terminology
//!
//! - Validator nickname: The nickname, a.k.a, Validator name in ChainX 1.0, is
//!     exclusively used as the ReferralId for getting some reward(10% of the
//!     total dividend) in Asset Mining because the depositor marks this
//!     validator as its referral when doing the deposit.
//!     Validator nickname and ReferralId is interchangeable in various scenarios.

#![cfg_attr(not(feature = "std"), no_std)]

mod constants;
mod election;
mod impls;
mod reward;
mod rpc;
mod slashing;
mod types;
pub mod weights;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
#[cfg(feature = "std")]
mod genesis;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::{
    ensure,
    log::debug,
    traits::{Currency, ExistenceRequirement, Get, LockableCurrency, WithdrawReasons},
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{
    traits::{Convert, SaturatedConversion, Saturating, StaticLookup, Zero},
    DispatchResult, Perbill,
};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use chainx_primitives::ReferralId;
use xp_mining_common::{Claim, ComputeMiningWeight, Delta, ZeroMiningWeightError};
use xp_mining_staking::{AssetMining, SessionIndex, UnbondedIndex};
use xpallet_support::traits::TreasuryAccount;

use crate::constants::*;

pub use self::impls::{IdentificationTuple, SimpleValidatorRewardPotAccountDeterminer};
pub use self::rpc::*;
pub use self::types::*;
pub use self::weights::WeightInfo;
pub use xp_mining_common::RewardPotAccountFor;

pub use pallet::*;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

        /// Get the treasury account.
        type TreasuryAccount: TreasuryAccount<Self::AccountId>;

        /// Asset mining integration.
        type AssetMining: AssetMining<BalanceOf<Self>>;

        /// Generate the reward pot account for a validator.
        type DetermineRewardPotAccount: RewardPotAccountFor<Self::AccountId, Self::AccountId>;

        /// Interface for interacting with a session module.
        type SessionInterface: self::SessionInterface<Self::AccountId>;

        /// The minimum byte length of validator referral id.
        #[pallet::constant]
        type MinimumReferralId: Get<u32>;

        /// The maximum byte length of validator referral id.
        #[pallet::constant]
        type MaximumReferralId: Get<u32>;

        /// An expected duration of the session.
        ///
        /// This parameter is used to determine the longevity of `heartbeat` transaction
        /// and a rough time when we should start considering sending heartbeats,
        /// since the workers avoids sending them at the very beginning of the session, assuming
        /// there is a chance the authority will produce a block and they won't be necessary.
        type SessionDuration: Get<Self::BlockNumber>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Nominate the `target` with `value` of the origin account's balance locked.
        #[pallet::weight(T::WeightInfo::bond())]
        pub fn bond(
            origin: OriginFor<T>,
            target: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::NotValidator);
            ensure!(
                value + Self::total_locked_of(&sender) <= Self::free_balance(&sender),
                Error::<T>::InsufficientBalance
            );
            if !Self::is_validator_bonding_itself(&sender, &target) {
                Self::check_validator_acceptable_votes_limit(&target, value)?;
            }

            Self::apply_bond(&sender, &target, value)?;
            Ok(())
        }

        /// Move the `value` of current nomination from one validator to another.
        #[pallet::weight(T::WeightInfo::rebond())]
        pub fn rebond(
            origin: OriginFor<T>,
            from: <T::Lookup as StaticLookup>::Source,
            to: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let from = T::Lookup::lookup(from)?;
            let to = T::Lookup::lookup(to)?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(
                Self::is_validator(&from) && Self::is_validator(&to),
                Error::<T>::NotValidator
            );
            ensure!(sender != from, Error::<T>::RebondSelfBondedNotAllowed);
            ensure!(
                value <= Self::bonded_to(&sender, &from),
                Error::<T>::InvalidRebondBalance
            );

            if !Self::is_validator_bonding_itself(&sender, &to) {
                Self::check_validator_acceptable_votes_limit(&to, value)?;
            }

            let current_block = <frame_system::Pallet<T>>::block_number();
            if let Some(last_rebond) = Self::last_rebond_of(&sender) {
                ensure!(
                    current_block > last_rebond + Self::bonding_duration(),
                    Error::<T>::NoMoreRebond
                );
            }

            Self::apply_rebond(&sender, &from, &to, value, current_block);
            Ok(())
        }

        /// Unnominate the `value` of bonded balance for validator `target`.
        #[pallet::weight(T::WeightInfo::unbond())]
        pub fn unbond(
            origin: OriginFor<T>,
            target: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            Self::can_unbond(&sender, &target, value)?;
            Self::apply_unbond(&sender, &target, value)?;
            Ok(())
        }

        /// Unlock the frozen unbonded balances that are due.
        #[pallet::weight(T::WeightInfo::unlock_unbonded_withdrawal())]
        pub fn unlock_unbonded_withdrawal(
            origin: OriginFor<T>,
            target: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] unbonded_index: UnbondedIndex,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            // TODO: use try_mutate
            let mut unbonded_chunks = Self::unbonded_chunks_of(&sender, &target);
            ensure!(!unbonded_chunks.is_empty(), Error::<T>::EmptyUnbondedChunks);
            ensure!(
                unbonded_index < unbonded_chunks.len() as u32,
                Error::<T>::InvalidUnbondedIndex
            );

            let Unbonded {
                value,
                locked_until,
            } = unbonded_chunks[unbonded_index as usize];
            let current_block = <frame_system::Pallet<T>>::block_number();
            ensure!(
                current_block > locked_until,
                Error::<T>::UnbondedWithdrawalNotYetDue
            );

            Self::apply_unlock_unbonded_withdrawal(&sender, value);

            unbonded_chunks.swap_remove(unbonded_index as usize);
            Nominations::<T>::mutate(&sender, &target, |nominator| {
                nominator.unbonded_chunks = unbonded_chunks;
            });

            Self::deposit_event(Event::<T>::Withdrawn(sender, value));
            Ok(())
        }

        /// Claim the staking reward given the `target` validator.
        #[pallet::weight(T::WeightInfo::claim())]
        pub fn claim(
            origin: OriginFor<T>,
            target: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            ensure!(Self::is_validator(&target), Error::<T>::NotValidator);

            <Self as Claim<T::AccountId>>::claim(&sender, &target)?;
            Ok(())
        }

        /// Declare the desire to validate for the origin account.
        #[pallet::weight(T::WeightInfo::validate())]
        pub fn validate(origin: OriginFor<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::NotValidator);
            Validators::<T>::mutate(sender, |validator| {
                validator.is_chilled = false;
            });
            Ok(())
        }

        /// Declare no desire to validate for the origin account.
        #[pallet::weight(T::WeightInfo::chill())]
        pub fn chill(origin: OriginFor<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::NotValidator);
            if Self::is_active(&sender) {
                ensure!(
                    Self::can_force_chilled(),
                    Error::<T>::TooFewActiveValidators
                );
            }
            Validators::<T>::mutate(sender, |validator| {
                validator.is_chilled = true;
                validator.last_chilled = Some(<frame_system::Pallet<T>>::block_number());
            });
            Ok(())
        }

        /// Register to be a validator for the origin account.
        ///
        /// The reason for using `validator_nickname` instead of `referral_id` as
        /// the variable name is when we interact with this interface from outside
        /// we can take this as the nickname of validator, which possibly
        /// can help reduce some misunderstanding for these unfamiliar with
        /// the referral mechanism in Asset Mining. In the context of codebase, we
        /// always use the concept of referral id.
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            validator_nickname: ReferralId,
            #[pallet::compact] initial_bond: BalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::check_referral_id(&validator_nickname)?;
            ensure!(!Self::is_validator(&sender), Error::<T>::AlreadyValidator);
            ensure!(
                (Self::validator_set().count() as u32) < MaximumValidatorCount::<T>::get(),
                Error::<T>::TooManyValidators
            );
            ensure!(
                initial_bond <= Self::free_balance(&sender),
                Error::<T>::InsufficientBalance
            );
            Self::apply_register(&sender, validator_nickname);
            if !initial_bond.is_zero() {
                Self::apply_bond(&sender, &sender, initial_bond)?;
            }
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::set_validator_count())]
        pub fn set_validator_count(
            origin: OriginFor<T>,
            #[pallet::compact] new: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ValidatorCount::<T>::put(new);
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::set_minimum_validator_count())]
        pub fn set_minimum_validator_count(
            origin: OriginFor<T>,
            #[pallet::compact] new: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;
            MinimumValidatorCount::<T>::put(new);
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::set_bonding_duration())]
        pub fn set_bonding_duration(
            origin: OriginFor<T>,
            #[pallet::compact] new: T::BlockNumber,
        ) -> DispatchResult {
            ensure_root(origin)?;
            BondingDuration::<T>::put(new);
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::set_validator_bonding_duration())]
        pub fn set_validator_bonding_duration(
            origin: OriginFor<T>,
            #[pallet::compact] new: T::BlockNumber,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ValidatorBondingDuration::<T>::put(new);
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::set_minimum_penalty())]
        pub fn set_minimum_penalty(
            origin: OriginFor<T>,
            #[pallet::compact] new: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            MinimumPenalty::<T>::put(new);
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::set_sessions_per_era())]
        pub fn set_sessions_per_era(
            origin: OriginFor<T>,
            #[pallet::compact] new: SessionIndex,
        ) -> DispatchResult {
            ensure_root(origin)?;
            SessionsPerEra::<T>::put(new);
            Ok(())
        }

        #[pallet::weight(10_000_000)]
        pub fn set_immortals(
            origin: OriginFor<T>,
            new: Vec<T::AccountId>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                new.iter().find(|&v| !Self::is_validator(v)).is_none(),
                Error::<T>::NotValidator
            );
            if new.is_empty() {
                Immortals::<T>::kill()
            } else {
                Immortals::<T>::put(new);
            }
            Ok(())
        }

        /// Clear the records in Staking for leaked `BondedWithdrawal` frozen balances.
        #[pallet::weight(T::WeightInfo::unlock_unbonded_withdrawal())]
        pub fn force_unlock_bonded_withdrawal(
            origin: OriginFor<T>,
            who: T::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Locks::<T>::mutate(&who, |locks| {
                locks.remove(&LockedType::BondedWithdrawal);
            });
            Self::purge_unlockings(&who);
            Self::deposit_event(Event::<T>::ForceAllWithdrawn(who));
            Ok(())
        }

        #[pallet::weight(10_000_000)]
        pub fn force_reset_staking_lock(
            origin: OriginFor<T>,
            accounts: Vec<T::AccountId>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            for who in accounts.iter() {
                Locks::<T>::mutate(who, |locks| {
                    locks.remove(&LockedType::BondedWithdrawal);
                    Self::purge_unlockings(who);
                    Self::set_lock(who, *locks.entry(LockedType::Bonded).or_default());
                });
            }
            Ok(())
        }

        #[pallet::weight(10_000_000)]
        pub fn force_set_lock(
            origin: OriginFor<T>,
            new_locks: Vec<(T::AccountId, BalanceOf<T>)>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            for (who, new_lock) in new_locks {
                Self::set_lock(&who, new_lock);
            }
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Issue new balance to this account. [account, reward_amount]
        Minted(T::AccountId, BalanceOf<T>),
        /// A validator (and its reward pot) was slashed. [validator, slashed_amount]
        Slashed(T::AccountId, BalanceOf<T>),
        /// A nominator bonded to the validator this amount. [nominator, validator, amount]
        Bonded(T::AccountId, T::AccountId, BalanceOf<T>),
        /// A nominator switched the vote from one validator to another. [nominator, from, to, amount]
        Rebonded(T::AccountId, T::AccountId, T::AccountId, BalanceOf<T>),
        /// A nominator unbonded this amount. [nominator, validator, amount]
        Unbonded(T::AccountId, T::AccountId, BalanceOf<T>),
        /// A nominator claimed the staking dividend. [nominator, validator, dividend]
        Claimed(T::AccountId, T::AccountId, BalanceOf<T>),
        /// The nominator withdrew the locked balance from the unlocking queue. [nominator, amount]
        Withdrawn(T::AccountId, BalanceOf<T>),
        /// Offenders were forcibly to be chilled due to insufficient reward pot balance. [session_index, chilled_validators]
        ForceChilled(SessionIndex, Vec<T::AccountId>),
        /// Unlock the unbonded withdrawal by force. [account]
        ForceAllWithdrawn(T::AccountId),
    }

    /// Old name generated by `decl_event`.
    #[deprecated(note = "use `Event` instead")]
    pub type RawEvent<T> = Event<T>;

    /// Error for the staking module.
    #[pallet::error]
    pub enum Error<T> {
        /// The operation of zero balance in Staking makes no sense.
        ZeroBalance,
        /// No rewards when the vote weight is zero.
        ZeroVoteWeight,
        /// Invalid validator target.
        NotValidator,
        /// The account is already registered as a validator.
        AlreadyValidator,
        /// The validators count already reaches `MaximumValidatorCount`.
        TooManyValidators,
        /// The validator can accept no more votes from other voters.
        NoMoreAcceptableVotes,
        /// The validator can not (forcedly) be chilled due to the limit of minimal validators count.
        TooFewActiveValidators,
        /// Free balance can not cover this bond operation.
        InsufficientBalance,
        /// Can not rebond due to the restriction of rebond frequency limit.
        NoMoreRebond,
        /// An account can only rebond the balance that is no more than what it has bonded to the validator.
        InvalidRebondBalance,
        /// Can not rebond the validator self-bonded votes as it has a much longer bonding duration.
        RebondSelfBondedNotAllowed,
        /// An account can only unbond the balance that is no more than what it has bonded to the validator.
        InvalidUnbondBalance,
        /// An account can have only `MaximumUnbondedChunkSize` unbonded entries in parallel.
        NoMoreUnbondChunks,
        /// The account has no unbonded entries.
        EmptyUnbondedChunks,
        /// Can not find the unbonded entry given the index.
        InvalidUnbondedIndex,
        /// The unbonded balances are still in the locked state.
        UnbondedWithdrawalNotYetDue,
        /// The length of referral identity is either too long or too short.
        InvalidReferralIdentityLength,
        /// The referral identity has been claimed by someone else.
        OccupiedReferralIdentity,
        /// Failed to pass the xss check.
        XssCheckFailed,
        /// Failed to allocate the dividend.
        AllocateDividendFailed,
    }

    /// The ideal number of staking participants.
    #[pallet::storage]
    #[pallet::getter(fn validator_count)]
    pub type ValidatorCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Minimum number of staking participants before emergency conditions are imposed.
    #[pallet::storage]
    #[pallet::getter(fn minimum_validator_count)]
    pub type MinimumValidatorCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultForMaximumValidatorCount() -> u32 {
        DEFAULT_MAXIMUM_VALIDATOR_COUNT
    }

    /// Maximum number of staking participants before emergency conditions are imposed.
    #[pallet::storage]
    #[pallet::getter(fn maximum_validator_count)]
    pub type MaximumValidatorCount<T: Config> =
        StorageValue<_, u32, ValueQuery, DefaultForMaximumValidatorCount>;

    /// Minimum value (self_bonded, total_bonded) to be a candidate of validator election.
    #[pallet::storage]
    #[pallet::getter(fn validator_candidate_requirement)]
    pub type ValidatorCandidateRequirement<T: Config> =
        StorageValue<_, BondRequirement<BalanceOf<T>>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultForSessionsPerEra() -> SessionIndex {
        12
    }

    /// The length of a staking era in sessions.
    #[pallet::storage]
    #[pallet::getter(fn sessions_per_era)]
    pub type SessionsPerEra<T: Config> =
        StorageValue<_, SessionIndex, ValueQuery, DefaultForSessionsPerEra>;

    #[pallet::type_value]
    pub fn DefaultForBondingDuration<T: Config>() -> T::BlockNumber {
        T::BlockNumber::saturated_from::<u64>(DEFAULT_BONDING_DURATION)
    }

    /// The length of the bonding duration in blocks.
    #[pallet::storage]
    #[pallet::getter(fn bonding_duration)]
    pub type BondingDuration<T: Config> =
        StorageValue<_, T::BlockNumber, ValueQuery, DefaultForBondingDuration<T>>;

    #[pallet::type_value]
    pub fn DefaultForValidatorBondingDuration<T: Config>() -> T::BlockNumber {
        T::BlockNumber::saturated_from::<u64>(DEFAULT_VALIDATOR_BONDING_DURATION)
    }

    /// The length of the bonding duration in blocks for validator.
    #[pallet::storage]
    #[pallet::getter(fn validator_bonding_duration)]
    pub type ValidatorBondingDuration<T: Config> =
        StorageValue<_, T::BlockNumber, ValueQuery, DefaultForValidatorBondingDuration<T>>;

    #[pallet::type_value]
    pub fn DefaultForMaximumUnbondedChunkSize() -> u32 {
        DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE
    }

    /// Maximum number of on-going unbonded chunk.
    #[pallet::storage]
    #[pallet::getter(fn maximum_unbonded_chunk_size)]
    pub type MaximumUnbondedChunkSize<T: Config> =
        StorageValue<_, u32, ValueQuery, DefaultForMaximumUnbondedChunkSize>;

    /// The beneficiary account of vesting schedule.
    #[pallet::storage]
    #[pallet::getter(fn vesting_account)]
    pub type VestingAccount<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    /// The validator account behind the referral id.
    #[pallet::storage]
    pub type ValidatorFor<T: Config> = StorageMap<_, Twox64Concat, ReferralId, T::AccountId>;

    #[pallet::type_value]
    pub fn DefaultForUpperBoundFactorOfAcceptableVotes() -> u32 {
        10u32
    }

    /// Maximum value of total_bonded/self_bonded.
    #[pallet::storage]
    #[pallet::getter(fn upper_bound_factor)]
    pub type UpperBoundFactorOfAcceptableVotes<T: Config> =
        StorageValue<_, u32, ValueQuery, DefaultForUpperBoundFactorOfAcceptableVotes>;

    /// (Treasury, Staking)
    #[pallet::storage]
    #[pallet::getter(fn global_distribution_ratio)]
    pub type GlobalDistributionRatio<T: Config> = StorageValue<_, GlobalDistribution, ValueQuery>;

    /// (Staker, Asset Miners)
    #[pallet::storage]
    #[pallet::getter(fn mining_distribution_ratio)]
    pub type MiningDistributionRatio<T: Config> = StorageValue<_, MiningDistribution, ValueQuery>;

    /// The map from (wannabe) validator key to the profile of that validator.
    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, ValidatorProfile<T::BlockNumber>, ValueQuery>;

    /// The map from validator key to the vote weight ledger of that validator.
    #[pallet::storage]
    #[pallet::getter(fn validator_ledgers)]
    pub type ValidatorLedgers<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        ValidatorLedger<BalanceOf<T>, VoteWeight, T::BlockNumber>,
        ValueQuery,
    >;

    /// The map from nominator to the vote weight ledger of all nominees.
    #[pallet::storage]
    #[pallet::getter(fn nominations)]
    pub type Nominations<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        T::AccountId,
        NominatorLedger<BalanceOf<T>, VoteWeight, T::BlockNumber>,
        ValueQuery,
    >;

    /// The map from nominator to the block number of last `rebond` operation.
    #[pallet::storage]
    #[pallet::getter(fn last_rebond_of)]
    pub type LastRebondOf<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::BlockNumber>;

    /// All kinds of locked balances of an account in Staking.
    #[pallet::storage]
    #[pallet::getter(fn locks)]
    pub type Locks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BTreeMap<LockedType, BalanceOf<T>>,
        ValueQuery,
    >;

    /// Mode of era forcing.
    #[pallet::storage]
    #[pallet::getter(fn force_era)]
    pub type ForceEra<T: Config> = StorageValue<_, Forcing, ValueQuery>;

    /// The current era index.
    ///
    /// This is the latest planned era, depending on how the Session pallet queues the validator
    /// set, it might be active or not.
    #[pallet::storage]
    #[pallet::getter(fn current_era)]
    pub type CurrentEra<T: Config> = StorageValue<_, EraIndex>;

    /// The active era information, it holds index and start.
    ///
    /// The active era is the era currently rewarded.
    /// Validator set of this era must be equal to `SessionInterface::validators`.
    #[pallet::storage]
    #[pallet::getter(fn active_era)]
    pub type ActiveEra<T: Config> = StorageValue<_, ActiveEraInfo>;

    /// The session index at which the era start for the last `HISTORY_DEPTH` eras.
    #[pallet::storage]
    #[pallet::getter(fn eras_start_session_index)]
    pub type ErasStartSessionIndex<T: Config> = StorageMap<_, Twox64Concat, EraIndex, SessionIndex>;

    #[pallet::type_value]
    pub fn DefaultForIsCurrentSessionFinal() -> bool {
        false
    }

    /// True if the current **planned** session is final. Note that this does not take era
    /// forcing into account.
    #[pallet::storage]
    #[pallet::getter(fn is_current_session_final)]
    pub type IsCurrentSessionFinal<T: Config> =
        StorageValue<_, bool, ValueQuery, DefaultForIsCurrentSessionFinal>;

    /// Offenders reported in last session.
    #[pallet::storage]
    #[pallet::getter(fn session_offenders)]
    pub(super) type SessionOffenders<T: Config> = StorageValue<_, BTreeMap<T::AccountId, Perbill>>;

    /// Minimum penalty for each slash.
    #[pallet::storage]
    #[pallet::getter(fn minimum_penalty)]
    pub type MinimumPenalty<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Immortal validators will always be elected if any.
    ///
    /// Immortals will be intialized from the genesis validators.
    #[pallet::storage]
    #[pallet::getter(fn immortals)]
    pub(super) type Immortals<T: Config> = StorageValue<_, Vec<T::AccountId>>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub validator_count: u32,
        pub minimum_validator_count: u32,
        pub maximum_validator_count: u32,
        pub sessions_per_era: SessionIndex,
        pub bonding_duration: T::BlockNumber,
        pub validator_bonding_duration: T::BlockNumber,
        pub maximum_unbonded_chunk_size: u32,
        pub vesting_account: T::AccountId,
        pub upper_bound_factor: u32,
        pub force_era: Forcing,
        pub minimum_penalty: BalanceOf<T>,
        pub validators: Vec<(T::AccountId, ReferralId, BalanceOf<T>)>,
        pub glob_dist_ratio: (u32, u32),
        pub mining_ratio: (u32, u32),
        pub candidate_requirement: (BalanceOf<T>, BalanceOf<T>),
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                validator_count: Default::default(),
                minimum_validator_count: Default::default(),
                maximum_validator_count: DEFAULT_MAXIMUM_VALIDATOR_COUNT,
                sessions_per_era: 12,
                bonding_duration: T::BlockNumber::saturated_from::<u64>(DEFAULT_BONDING_DURATION),
                validator_bonding_duration: T::BlockNumber::saturated_from::<u64>(
                    DEFAULT_VALIDATOR_BONDING_DURATION,
                ),
                maximum_unbonded_chunk_size: DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE,
                vesting_account: Default::default(),
                upper_bound_factor: 10u32,
                force_era: Default::default(),
                minimum_penalty: Default::default(),
                validators: Default::default(),
                glob_dist_ratio: Default::default(),
                mining_ratio: Default::default(),
                candidate_requirement: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <ValidatorCount<T>>::put(self.validator_count);
            <MinimumValidatorCount<T>>::put(self.minimum_validator_count);
            <MaximumValidatorCount<T>>::put(self.maximum_validator_count);
            <SessionsPerEra<T>>::put(self.sessions_per_era);
            <BondingDuration<T>>::put(self.bonding_duration);
            <ValidatorBondingDuration<T>>::put(self.validator_bonding_duration);
            <MaximumUnbondedChunkSize<T>>::put(self.maximum_unbonded_chunk_size);
            <VestingAccount<T>>::put(self.vesting_account.clone());
            <UpperBoundFactorOfAcceptableVotes<T>>::put(self.upper_bound_factor);
            <ForceEra<T>>::put(self.force_era);
            <MinimumPenalty<T>>::put(self.minimum_penalty);

            let extra_genesis_builder: fn(&Self) = |config: &GenesisConfig<T>| {
                assert!(config.glob_dist_ratio.0 + config.glob_dist_ratio.1 > 0);
                assert!(config.mining_ratio.0 + config.mining_ratio.1 > 0);
                <GlobalDistributionRatio<T>>::put(GlobalDistribution {
                    treasury: config.glob_dist_ratio.0,
                    mining: config.glob_dist_ratio.1,
                });
                <MiningDistributionRatio<T>>::put(MiningDistribution {
                    asset: config.mining_ratio.0,
                    staking: config.mining_ratio.1,
                });
                ValidatorCandidateRequirement::<T>::put(BondRequirement {
                    self_bonded: config.candidate_requirement.0,
                    total: config.candidate_requirement.1,
                });
                Immortals::<T>::put(
                    config
                        .validators
                        .iter()
                        .map(|(validator, _, _)| validator.clone())
                        .collect::<Vec<_>>(),
                );
                for (validator, referral_id, balance) in &config.validators {
                    assert!(
                        Pallet::<T>::free_balance(validator) >= *balance,
                        "Validator does not have enough balance to bond."
                    );
                    Pallet::<T>::check_referral_id(referral_id)
                        .expect("Validator referral id must be valid; qed");
                    Pallet::<T>::apply_register(validator, referral_id.to_vec());
                    Pallet::<T>::apply_bond(validator, validator, *balance)
                        .expect("Bonding to validator itself can not fail; qed");
                }
            };
            extra_genesis_builder(self);
        }
    }
}

impl<T: Config> From<ZeroMiningWeightError> for Error<T> {
    fn from(_: ZeroMiningWeightError) -> Self {
        Self::ZeroVoteWeight
    }
}

/// Means for interacting with a specialized version of the `session` trait.
///
/// This is needed because `Staking` sets the `ValidatorIdOf` of the `pallet_session::Config`
pub trait SessionInterface<AccountId>: frame_system::Config {
    /// Disable a given validator by stash ID.
    ///
    /// Returns `true` if new era should be forced at the end of this session.
    /// This allows preventing a situation where there is too many validators
    /// disabled and block production stalls.
    fn disable_validator(validator: &AccountId) -> Result<bool, ()>;

    /// Get the validators from session.
    fn validators() -> Vec<AccountId>;
}

impl<T: Config> SessionInterface<<T as frame_system::Config>::AccountId> for T
where
    T: pallet_session::Config<ValidatorId = <T as frame_system::Config>::AccountId>,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Config>::AccountId>,
    T::SessionManager: pallet_session::SessionManager<<T as frame_system::Config>::AccountId>,
    T::ValidatorIdOf: Convert<
        <T as frame_system::Config>::AccountId,
        Option<<T as frame_system::Config>::AccountId>,
    >,
{
    fn disable_validator(validator: &<T as frame_system::Config>::AccountId) -> Result<bool, ()> {
        <pallet_session::Pallet<T>>::disable(validator)
    }

    fn validators() -> Vec<<T as frame_system::Config>::AccountId> {
        <pallet_session::Pallet<T>>::validators()
    }
}

impl<T: Config> xpallet_support::traits::Validator<T::AccountId> for Pallet<T> {
    fn is_validator(who: &T::AccountId) -> bool {
        Self::is_validator(who)
    }

    fn validator_for(name: &[u8]) -> Option<T::AccountId> {
        Self::validator_for(name)
    }
}

impl<T: Config> Pallet<T> {
    /// Returns true if the account `who` is a validator.
    #[inline]
    pub fn is_validator(who: &T::AccountId) -> bool {
        Validators::<T>::contains_key(who)
    }

    /// Returns the (possible) validator account behind the given referral id.
    #[inline]
    pub fn validator_for(referral_id: &[u8]) -> Option<T::AccountId> {
        ValidatorFor::<T>::get(referral_id)
    }

    /// Return true if the validator `who` is chilled.
    #[inline]
    pub fn is_chilled(who: &T::AccountId) -> bool {
        Validators::<T>::get(who).is_chilled
    }

    /// Return true if the validator `who` is not chilled.
    #[inline]
    pub fn is_active(who: &T::AccountId) -> bool {
        !Self::is_chilled(who)
    }

    /// Returns all the registered validators in Staking.
    #[inline]
    pub fn validator_set() -> impl Iterator<Item = T::AccountId> {
        Validators::<T>::iter().map(|(v, _)| v)
    }

    /// Returns all the validators that are not chilled.
    #[inline]
    pub fn active_validator_set() -> impl Iterator<Item = T::AccountId> {
        Self::validator_set().filter(Self::is_active)
    }

    /// Returns the sum of total active staked PCX, i.e., total staking power.
    ///
    /// * One (indivisible) PCX one power.
    /// * Only the votes of active validators are counted.
    #[inline]
    pub fn total_staked() -> BalanceOf<T> {
        Self::active_validator_votes().fold(Zero::zero(), |acc: BalanceOf<T>, (_, x)| acc + x)
    }

    /// Returns the bonded balance of Staking for the given account.
    pub fn staked_of(who: &T::AccountId) -> BalanceOf<T> {
        *Self::locks(who).entry(LockedType::Bonded).or_default()
    }

    /// Returns the associated reward pot account for the given validator.
    #[inline]
    pub fn reward_pot_for(validator: &T::AccountId) -> T::AccountId {
        T::DetermineRewardPotAccount::reward_pot_account_for(validator)
    }

    #[inline]
    fn unbonded_chunks_of(
        nominator: &T::AccountId,
        target: &T::AccountId,
    ) -> Vec<Unbonded<BalanceOf<T>, T::BlockNumber>> {
        Nominations::<T>::get(nominator, target).unbonded_chunks
    }

    #[inline]
    fn free_balance(who: &T::AccountId) -> BalanceOf<T> {
        T::Currency::free_balance(who)
    }

    /// Returns the total votes of a validator.
    #[inline]
    fn total_votes_of(validator: &T::AccountId) -> BalanceOf<T> {
        ValidatorLedgers::<T>::get(validator).total_nomination
    }

    /// Returns the balance of `nominator` has voted to `nominee`.
    #[inline]
    fn bonded_to(nominator: &T::AccountId, nominee: &T::AccountId) -> BalanceOf<T> {
        Nominations::<T>::get(nominator, nominee).nomination
    }

    #[inline]
    fn transfer(from: &T::AccountId, to: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        T::Currency::transfer(from, to, value, ExistenceRequirement::KeepAlive)
    }

    /// Create/Update/Remove a new balance lock on account `who`.
    #[inline]
    fn set_lock(who: &T::AccountId, new_locked: BalanceOf<T>) {
        if new_locked.is_zero() {
            T::Currency::remove_lock(STAKING_ID, who);
        } else {
            T::Currency::set_lock(STAKING_ID, who, new_locked, WithdrawReasons::all());
        }
    }

    fn purge_unlockings(who: &T::AccountId) {
        for (target, _) in Nominations::<T>::iter_prefix(who) {
            Nominations::<T>::mutate(&who, &target, |nominator| {
                nominator.unbonded_chunks.clear();
            });
        }
    }

    /// Returns an iterator of tuple (active_validator, total_votes_of_this_validator).
    ///
    /// Only these active validators are able to be rewarded on each new session,
    /// the inactive ones earn nothing.
    pub fn active_validator_votes() -> impl Iterator<Item = (T::AccountId, BalanceOf<T>)> {
        Self::active_validator_set().map(|v| {
            let total_votes = Self::total_votes_of(&v);
            (v, total_votes)
        })
    }

    /// Returns the balance that validator bonded to itself.
    fn validator_self_bonded(validator: &T::AccountId) -> BalanceOf<T> {
        Self::bonded_to(validator, validator)
    }

    fn is_validator_bonding_itself(nominator: &T::AccountId, nominee: &T::AccountId) -> bool {
        Self::is_validator(nominator) && *nominator == *nominee
    }

    fn acceptable_votes_limit_of(validator: &T::AccountId) -> BalanceOf<T> {
        Self::validator_self_bonded(validator) * BalanceOf::<T>::from(Self::upper_bound_factor())
    }

    fn check_referral_id(referral_id: &[u8]) -> Result<(), Error<T>> {
        let referral_id_len = referral_id.len();
        ensure!(
            referral_id_len >= T::MinimumReferralId::get() as usize
                && referral_id_len <= T::MaximumReferralId::get() as usize,
            Error::<T>::InvalidReferralIdentityLength
        );
        ensure!(
            xp_runtime::xss_check(referral_id).is_ok(),
            Error::<T>::XssCheckFailed
        );
        ensure!(
            Self::validator_for(referral_id).is_none(),
            Error::<T>::OccupiedReferralIdentity
        );
        Ok(())
    }

    /// Returns Ok if the validator can still accept the `value` of new votes.
    fn check_validator_acceptable_votes_limit(
        validator: &T::AccountId,
        value: BalanceOf<T>,
    ) -> Result<(), Error<T>> {
        let cur_total = Self::total_votes_of(validator);
        let upper_limit = Self::acceptable_votes_limit_of(validator);
        if cur_total + value <= upper_limit {
            Ok(())
        } else {
            Err(Error::<T>::NoMoreAcceptableVotes)
        }
    }

    /// Ensures that at the end of the current session there will be a new era.
    fn ensure_new_era() {
        match ForceEra::<T>::get() {
            Forcing::ForceAlways | Forcing::ForceNew => (),
            _ => ForceEra::<T>::put(Forcing::ForceNew),
        }
    }

    /// At least one validator is required.
    fn reasonable_minimum_validator_count() -> u32 {
        Self::minimum_validator_count().max(1)
    }

    /// Returns true if the number of active validators are more than the
    /// reasonable minimum validator count.
    fn can_force_chilled() -> bool {
        let mut active_cnt = 0u32;
        let minimum_validator_cnt = Self::reasonable_minimum_validator_count();
        Self::validator_set()
            .try_for_each(|v| {
                if Self::is_active(&v) {
                    active_cnt += 1;
                }
                if active_cnt > minimum_validator_cnt {
                    Err(())
                } else {
                    Ok(())
                }
            })
            .is_err()
    }

    fn try_force_chilled(who: &T::AccountId) -> Result<(), Error<T>> {
        if !Self::can_force_chilled() {
            return Err(Error::<T>::TooFewActiveValidators);
        }
        Self::apply_force_chilled(who);
        Ok(())
    }

    /// Force the validator `who` to be chilled.
    fn apply_force_chilled(who: &T::AccountId) {
        Validators::<T>::mutate(who, |validator| {
            validator.is_chilled = true;
            validator.last_chilled = Some(<frame_system::Pallet<T>>::block_number());
        });
    }

    /// Set a lock on `value` of free balance of an account.
    pub(crate) fn bond_reserve(who: &T::AccountId, value: BalanceOf<T>) {
        Locks::<T>::mutate(who, |locks| {
            *locks.entry(LockedType::Bonded).or_default() += value;

            let staking_locked = locks
                .values()
                .fold(Zero::zero(), |acc: BalanceOf<T>, x| acc + *x);
            Self::set_lock(who, staking_locked);
        });
    }

    fn can_unbond(
        sender: &T::AccountId,
        target: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
        ensure!(Self::is_validator(target), Error::<T>::NotValidator);
        ensure!(
            value <= Self::bonded_to(sender, target),
            Error::<T>::InvalidUnbondBalance
        );
        ensure!(
            Self::unbonded_chunks_of(sender, target).len()
                < Self::maximum_unbonded_chunk_size() as usize,
            Error::<T>::NoMoreUnbondChunks
        );
        Ok(())
    }

    /// `unbond` only triggers the internal change of Staking locked type.
    fn unbond_reserve(who: &T::AccountId, value: BalanceOf<T>) -> Result<(), Error<T>> {
        Locks::<T>::mutate(who, |locks| {
            *locks.entry(LockedType::Bonded).or_default() -= value;
            *locks.entry(LockedType::BondedWithdrawal).or_default() += value;
        });
        Ok(())
    }

    /// Returns the total locked balances in Staking.
    fn total_locked_of(who: &T::AccountId) -> BalanceOf<T> {
        Self::locks(who)
            .values()
            .fold(Zero::zero(), |acc: BalanceOf<T>, x| acc + *x)
    }

    /// Settles and update the vote weight state of the nominator `source` and
    /// validator `target` given the delta amount.
    fn update_vote_weight(
        source: &T::AccountId,
        target: &T::AccountId,
        delta: Delta<BalanceOf<T>>,
    ) {
        let current_block = <frame_system::Pallet<T>>::block_number();

        let source_weight =
            <Self as ComputeMiningWeight<T::AccountId, T::BlockNumber>>::settle_claimer_weight(
                source,
                target,
                current_block,
            );

        let target_weight =
            <Self as ComputeMiningWeight<T::AccountId, T::BlockNumber>>::settle_claimee_weight(
                target,
                current_block,
            );

        Self::set_nominator_vote_weight(source, target, source_weight, current_block, delta);
        Self::set_validator_vote_weight(target, target_weight, current_block, delta);
    }

    fn apply_register(who: &T::AccountId, referral_id: ReferralId) {
        let current_block = <frame_system::Pallet<T>>::block_number();
        ValidatorFor::<T>::insert(&referral_id, who.clone());
        Validators::<T>::insert(
            who,
            ValidatorProfile {
                registered_at: current_block,
                referral_id,
                ..Default::default()
            },
        );
    }

    fn apply_bond(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Self::bond_reserve(nominator, value);
        Self::update_vote_weight(nominator, nominee, Delta::Add(value));
        Self::deposit_event(Event::<T>::Bonded(
            nominator.clone(),
            nominee.clone(),
            value,
        ));
        Ok(())
    }

    fn apply_rebond(
        who: &T::AccountId,
        from: &T::AccountId,
        to: &T::AccountId,
        value: BalanceOf<T>,
        current_block: T::BlockNumber,
    ) {
        Self::update_vote_weight(who, from, Delta::Sub(value));
        Self::update_vote_weight(who, to, Delta::Add(value));
        LastRebondOf::<T>::mutate(who, |last_rebond| {
            *last_rebond = Some(current_block);
        });
        Self::deposit_event(Event::<T>::Rebonded(
            who.clone(),
            from.clone(),
            to.clone(),
            value,
        ));
    }

    fn mutate_unbonded_chunks(
        who: &T::AccountId,
        target: &T::AccountId,
        value: BalanceOf<T>,
        locked_until: T::BlockNumber,
    ) {
        Nominations::<T>::mutate(who, target, |nominator| {
            if let Some(idx) = nominator
                .unbonded_chunks
                .iter()
                .position(|x| x.locked_until == locked_until)
            {
                nominator.unbonded_chunks[idx].value += value;
            } else {
                nominator.unbonded_chunks.push(Unbonded {
                    value,
                    locked_until,
                });
            }
        });
    }

    fn bonding_duration_for(who: &T::AccountId, target: &T::AccountId) -> T::BlockNumber {
        if Self::is_validator(who) && *who == *target {
            Self::validator_bonding_duration()
        } else {
            Self::bonding_duration()
        }
    }

    fn apply_unbond(
        who: &T::AccountId,
        target: &T::AccountId,
        value: BalanceOf<T>,
    ) -> Result<(), Error<T>> {
        debug!(
            target: "runtime::mining::staking",
            "[apply_unbond] who:{:?}, target:{:?}, value:{:?}",
            who, target, value
        );
        Self::unbond_reserve(who, value)?;

        let locked_until =
            <frame_system::Pallet<T>>::block_number() + Self::bonding_duration_for(who, target);
        Self::mutate_unbonded_chunks(who, target, value, locked_until);

        Self::update_vote_weight(who, target, Delta::Sub(value));

        Self::deposit_event(Event::<T>::Unbonded(who.clone(), target.clone(), value));

        Ok(())
    }

    fn apply_unlock_unbonded_withdrawal(who: &T::AccountId, value: BalanceOf<T>) {
        let new_bonded = Self::total_locked_of(who) - value;
        Self::set_lock(who, new_bonded);
        Locks::<T>::mutate(who, |locks| {
            let old_value = *locks.entry(LockedType::BondedWithdrawal).or_default();
            // All the bonded funds have been withdrawn.
            if old_value == value {
                locks.remove(&LockedType::BondedWithdrawal);
            } else {
                locks.insert(
                    LockedType::BondedWithdrawal,
                    old_value.saturating_sub(value),
                );
            }
        });
    }
}
