// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! # Staking Module
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
mod weight_info;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use sp_std::prelude::*;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    storage::IterableStorageMap,
    traits::{Currency, ExistenceRequirement, Get, LockableCurrency, WithdrawReasons},
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{
    traits::{Convert, SaturatedConversion, Saturating, StaticLookup, Zero},
    DispatchResult,
};
use sp_std::collections::btree_map::BTreeMap;

use chainx_primitives::ReferralId;
pub use xp_mining_common::RewardPotAccountFor;
use xp_mining_common::{Claim, ComputeMiningWeight, Delta, ZeroMiningWeightError};
use xp_mining_staking::{AssetMining, SessionIndex, UnbondedIndex};
use xpallet_support::{debug, traits::TreasuryAccount};

use self::constants::*;
pub use self::impls::{IdentificationTuple, SimpleValidatorRewardPotAccountDeterminer};
pub use self::rpc::*;
pub use self::types::*;
pub use self::weight_info::WeightInfo;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

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

    /// The number of unfinished sessions in the first halving epoch.
    ///
    /// When the ChainX 2.0 migration happens, the first halving epoch is not over yet.
    type MigrationSessionOffset: Get<SessionIndex>;

    /// The minimum byte length of validator referral id.
    type MinimumReferralId: Get<u32>;

    /// The maximum byte length of validator referral id.
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

decl_storage! {
    trait Store for Module<T: Trait> as XStaking {
        /// The ideal number of staking participants.
        pub ValidatorCount get(fn validator_count) config(): u32;

        /// Minimum number of staking participants before emergency conditions are imposed.
        pub MinimumValidatorCount get(fn minimum_validator_count) config(): u32;

        /// Maximum number of staking participants before emergency conditions are imposed.
        pub MaximumValidatorCount get(fn maximum_validator_count) config():
            u32 = DEFAULT_MAXIMUM_VALIDATOR_COUNT;

        /// Minimum value (self_bonded, total_bonded) to be a candidate of validator election.
        pub ValidatorCandidateRequirement get(fn validator_bond_requirement):
            BondRequirement<BalanceOf<T>>;

        /// The length of a staking era in sessions.
        pub SessionsPerEra get(fn sessions_per_era) config(): SessionIndex = 12;

        /// The length of the bonding duration in blocks.
        pub BondingDuration get(fn bonding_duration) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(DEFAULT_BONDING_DURATION);

        /// The length of the bonding duration in blocks for validator.
        pub ValidatorBondingDuration get(fn validator_bonding_duration) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(DEFAULT_VALIDATOR_BONDING_DURATION);

        /// Maximum number of on-going unbonded chunk.
        pub MaximumUnbondedChunkSize get(fn maximum_unbonded_chunk_size) config():
            u32 = DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE;

        /// The beneficiary account of vesting schedule.
        pub VestingAccount get(fn vesting_account) config(): T::AccountId;

        /// The validator account behind the referral id.
        pub ValidatorFor: map hasher(twox_64_concat) ReferralId => Option<T::AccountId>;

        /// Maximum value of total_bonded/self_bonded.
        pub UpperBoundFactorOfAcceptableVotes get(fn upper_bound_factor) config():
            u32 = 10u32;

        /// (Treasury, Staking)
        pub GlobalDistributionRatio get(fn global_distribution_ratio): GlobalDistribution;

        /// (Staker, Asset Miners)
        pub MiningDistributionRatio get(fn mining_distribution_ratio): MiningDistribution;

        /// The map from (wannabe) validator key to the profile of that validator.
        pub Validators get(fn validators):
            map hasher(twox_64_concat) T::AccountId => ValidatorProfile<T::BlockNumber>;

        /// The map from validator key to the vote weight ledger of that validator.
        pub ValidatorLedgers get(fn validator_ledgers):
            map hasher(twox_64_concat) T::AccountId => ValidatorLedger<BalanceOf<T>, VoteWeight, T::BlockNumber>;

        /// The map from nominator to the vote weight ledger of all nominees.
        pub Nominations get(fn nominations):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) T::AccountId
            => NominatorLedger<BalanceOf<T>, VoteWeight, T::BlockNumber>;

        /// The map from nominator to the block number of last `rebond` operation.
        pub LastRebondOf get(fn last_rebond_of):
            map hasher(twox_64_concat) T::AccountId => Option<T::BlockNumber>;

        /// All kinds of locked balances of an account in Staking.
        pub Locks get(fn locks):
            map hasher(blake2_128_concat) T::AccountId => BTreeMap<LockedType, BalanceOf<T>>;

        /// Mode of era forcing.
        pub ForceEra get(fn force_era) config(): Forcing;

        /// The current era index.
        ///
        /// This is the latest planned era, depending on how the Session pallet queues the validator
        /// set, it might be active or not.
        pub CurrentEra get(fn current_era): Option<EraIndex>;

        /// The active era information, it holds index and start.
        ///
        /// The active era is the era currently rewarded.
        /// Validator set of this era must be equal to `SessionInterface::validators`.
        pub ActiveEra get(fn active_era): Option<ActiveEraInfo>;

        /// The session index at which the era start for the last `HISTORY_DEPTH` eras.
        pub ErasStartSessionIndex get(fn eras_start_session_index):
            map hasher(twox_64_concat) EraIndex => Option<SessionIndex>;

        /// True if the current **planned** session is final. Note that this does not take era
        /// forcing into account.
        pub IsCurrentSessionFinal get(fn is_current_session_final): bool = false;

        /// Offenders reported in current session.
        OffendersInSession get(fn offenders_in_session): Vec<T::AccountId>;

        /// Minimum penalty for each slash.
        pub MinimumPenalty get(fn minimum_penalty) config(): BalanceOf<T>;

        /// The higher the severity, the more slash for the offences.
        pub OffenceSeverity get(fn offence_severity) config(): u32;
    }

    add_extra_genesis {
        // Staking validators are used for initializing the genesis easier in tests.
        // For the mainnet genesis, use `Module::<T>::initialize_validators()`.
        config(validators): Vec<(T::AccountId, ReferralId, BalanceOf<T>)>;
        config(glob_dist_ratio): (u32, u32);
        config(mining_ratio): (u32, u32);
        build(|config: &GenesisConfig<T>| {
            assert!(config.offence_severity > 1, "Offence severity too weak");
            assert!(config.glob_dist_ratio.0 + config.glob_dist_ratio.1 > 0);
            assert!(config.mining_ratio.0 + config.mining_ratio.1 > 0);
            GlobalDistributionRatio::put(GlobalDistribution {
                treasury: config.glob_dist_ratio.0,
                mining: config.glob_dist_ratio.1,
            });
            MiningDistributionRatio::put(MiningDistribution {
                asset: config.mining_ratio.0,
                staking: config.mining_ratio.1,
            });

            for (validator, referral_id, balance) in &config.validators {
                assert!(
                    Module::<T>::free_balance(validator) >= *balance,
                    "Validator does not have enough balance to bond."
                );
                Module::<T>::check_referral_id(referral_id)
                    .expect("Validator referral id must be valid; qed");
                Module::<T>::apply_register(validator, referral_id.to_vec());
                Module::<T>::apply_bond(validator, validator, *balance)
                    .expect("Bonding to validator itself can not fail; qed");
            }
        });
    }
}

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as frame_system::Trait>::AccountId
    {
        /// Issue new balance to this account. [account, reward_amount]
        Minted(AccountId, Balance),
        /// A validator (and its reward pot) was slashed. [validator, slashed_amount]
        Slashed(AccountId, Balance),
        /// A nominator bonded to the validator this amount. [nominator, validator, amount]
        Bonded(AccountId, AccountId, Balance),
        /// A nominator switched the vote from one validator to another. [nominator, from, to, amount]
        Rebonded(AccountId, AccountId, AccountId, Balance),
        /// A nominator unbonded this amount. [nominator, validator, amount]
        Unbonded(AccountId, AccountId, Balance),
        /// A nominator claimed the staking dividend. [nominator, validator, dividend]
        Claimed(AccountId, AccountId, Balance),
        /// The nominator withdrew the locked balance from the unlocking queue. [nominator, amount]
        Withdrawn(AccountId, Balance),
        /// Offenders were forcibly to be chilled due to insufficient reward pot balance. [session_index, chilled_validators]
        ForceChilled(SessionIndex, Vec<AccountId>),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
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
        /// Offence severity is too weak to kick out an offender.
        WeakOffenceSeverity,
    }
}

impl<T: Trait> From<ZeroMiningWeightError> for Error<T> {
    fn from(_: ZeroMiningWeightError) -> Self {
        Self::ZeroVoteWeight
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// The minimum byte length of referral id.
        const MinimumReferralId: u32 = T::MinimumReferralId::get();

        /// The maximum byte length of referral id.
        const MaximumReferralId: u32 = T::MaximumReferralId::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Nominate the `target` with `value` of the origin account's balance locked.
        #[weight = T::WeightInfo::bond()]
        pub fn bond(origin, target: <T::Lookup as StaticLookup>::Source, #[compact] value: BalanceOf<T>) {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::NotValidator);
            ensure!(value <= Self::free_balance(&sender), Error::<T>::InsufficientBalance);
            if !Self::is_validator_bonding_itself(&sender, &target) {
                Self::check_validator_acceptable_votes_limit(&target, value)?;
            }

            Self::apply_bond(&sender, &target, value)?;
        }

        /// Move the `value` of current nomination from one validator to another.
        #[weight = T::WeightInfo::rebond()]
        fn rebond(origin, from: <T::Lookup as StaticLookup>::Source, to: <T::Lookup as StaticLookup>::Source, #[compact] value: BalanceOf<T>) {
            let sender = ensure_signed(origin)?;
            let from = T::Lookup::lookup(from)?;
            let to = T::Lookup::lookup(to)?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&from) && Self::is_validator(&to), Error::<T>::NotValidator);
            ensure!(sender != from, Error::<T>::RebondSelfBondedNotAllowed);
            ensure!(value <= Self::bonded_to(&sender, &from), Error::<T>::InvalidRebondBalance);

            if !Self::is_validator_bonding_itself(&sender, &to) {
                Self::check_validator_acceptable_votes_limit(&to, value)?;
            }

            let current_block = <frame_system::Module<T>>::block_number();
            if let Some(last_rebond) = Self::last_rebond_of(&sender) {
                ensure!(
                    current_block > last_rebond + Self::bonding_duration(),
                    Error::<T>::NoMoreRebond
                );
            }

            Self::apply_rebond(&sender,  &from, &to, value, current_block);
        }

        /// Unnominate the `value` of bonded balance for validator `target`.
        #[weight = T::WeightInfo::unbond()]
        fn unbond(origin, target: <T::Lookup as StaticLookup>::Source, #[compact] value: BalanceOf<T>) {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            Self::can_unbond(&sender, &target, value)?;
            Self::apply_unbond(&sender, &target, value)?;
        }

        /// Unlock the frozen unbonded balances that are due.
        #[weight = T::WeightInfo::unlock_unbonded_withdrawal()]
        fn unlock_unbonded_withdrawal(
            origin,
            target: <T::Lookup as StaticLookup>::Source,
            #[compact] unbonded_index: UnbondedIndex
        ) {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            // TODO: use try_mutate
            let mut unbonded_chunks = Self::unbonded_chunks_of(&sender, &target);
            ensure!(!unbonded_chunks.is_empty(), Error::<T>::EmptyUnbondedChunks);
            ensure!(unbonded_index < unbonded_chunks.len() as u32, Error::<T>::InvalidUnbondedIndex);

            let Unbonded { value, locked_until } = unbonded_chunks[unbonded_index as usize];
            let current_block = <frame_system::Module<T>>::block_number();

            ensure!(current_block > locked_until, Error::<T>::UnbondedWithdrawalNotYetDue);

            Self::apply_unlock_unbonded_withdrawal(&sender, value);

            unbonded_chunks.swap_remove(unbonded_index as usize);
            Nominations::<T>::mutate(&sender, &target, |nominator| {
                nominator.unbonded_chunks = unbonded_chunks;
            });

            Self::deposit_event(Event::<T>::Withdrawn(sender, value));
        }

        /// Claim the staking reward given the `target` validator.
        #[weight = T::WeightInfo::claim()]
        fn claim(origin, target: <T::Lookup as StaticLookup>::Source) {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            ensure!(Self::is_validator(&target), Error::<T>::NotValidator);

            <Self as Claim<T::AccountId>>::claim(&sender, &target)?;
        }

        /// Declare the desire to validate for the origin account.
        #[weight = T::WeightInfo::validate()]
        fn validate(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::NotValidator);
            Validators::<T>::mutate(sender, |validator| {
                    validator.is_chilled = false;
                }
            );
        }

        /// Declare no desire to validate for the origin account.
        #[weight = T::WeightInfo::chill()]
        fn chill(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::NotValidator);
            if Self::is_active(&sender) {
                ensure!(Self::can_force_chilled(), Error::<T>::TooFewActiveValidators);
            }
            Validators::<T>::mutate(sender, |validator| {
                    validator.is_chilled = true;
                    validator.last_chilled = Some(<frame_system::Module<T>>::block_number());
                }
            );
        }

        /// Register to be a validator for the origin account.
        ///
        /// The reason for using `validator_nickname` instead of `referral_id` as
        /// the variable name is when we interact with this interface from outside
        /// we can take this as the nickname of validator, which possibly
        /// can help reduce some misunderstanding for these unfamiliar with
        /// the referral mechanism in Asset Mining. In the context of codebase, we
        /// always use the concept of referral id.
        #[weight = T::WeightInfo::register()]
        pub fn register(origin, validator_nickname: ReferralId, #[compact] initial_bond: BalanceOf<T>) {
            let sender = ensure_signed(origin)?;
            Self::check_referral_id(&validator_nickname)?;
            ensure!(!Self::is_validator(&sender), Error::<T>::AlreadyValidator);
            ensure!(
                (Self::validator_set().count() as u32) < MaximumValidatorCount::get(),
                Error::<T>::TooManyValidators
            );
            ensure!(initial_bond <= Self::free_balance(&sender), Error::<T>::InsufficientBalance);
            Self::apply_register(&sender, validator_nickname);
            if !initial_bond.is_zero() {
                Self::apply_bond(&sender, &sender, initial_bond)?;
            }
        }

        #[weight = T::WeightInfo::set_validator_count()]
        fn set_validator_count(origin, #[compact] new: u32) {
            ensure_root(origin)?;
            ValidatorCount::put(new);
        }

        #[weight = T::WeightInfo::set_minimum_validator_count()]
        fn set_minimum_validator_count(origin, #[compact] new: u32) {
            ensure_root(origin)?;
            MinimumValidatorCount::put(new);
        }

        #[weight = T::WeightInfo::set_bonding_duration()]
        fn set_bonding_duration(origin, #[compact] new: T::BlockNumber) {
            ensure_root(origin)?;
            BondingDuration::<T>::put(new);
        }

        #[weight = T::WeightInfo::set_validator_bonding_duration()]
        fn set_validator_bonding_duration(origin, #[compact] new: T::BlockNumber) {
            ensure_root(origin)?;
            ValidatorBondingDuration::<T>::put(new);
        }

        // FIXME: add to WeightInfo once it's stable.
        #[weight = 10_000_000]
        fn set_minimum_penalty(origin, #[compact] new: BalanceOf<T>) {
            ensure_root(origin)?;
            MinimumPenalty::<T>::put(new);
        }

        #[weight = 10_000_000]
        fn set_sessions_per_era(origin, #[compact] new: SessionIndex) {
            ensure_root(origin)?;
            SessionsPerEra::put(new);
        }

        #[weight = 10_000_000]
        fn set_offence_severity(origin, #[compact] new: u32) {
            ensure_root(origin)?;
            ensure!(new > 1, Error::<T>::WeakOffenceSeverity);
            OffenceSeverity::put(new);
        }
    }
}

/// Means for interacting with a specialized version of the `session` trait.
///
/// This is needed because `Staking` sets the `ValidatorIdOf` of the `pallet_session::Trait`
pub trait SessionInterface<AccountId>: frame_system::Trait {
    /// Disable a given validator by stash ID.
    ///
    /// Returns `true` if new era should be forced at the end of this session.
    /// This allows preventing a situation where there is too many validators
    /// disabled and block production stalls.
    fn disable_validator(validator: &AccountId) -> Result<bool, ()>;

    /// Get the validators from session.
    fn validators() -> Vec<AccountId>;
}

impl<T: Trait> SessionInterface<<T as frame_system::Trait>::AccountId> for T
where
    T: pallet_session::Trait<ValidatorId = <T as frame_system::Trait>::AccountId>,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Trait>::AccountId>,
    T::SessionManager: pallet_session::SessionManager<<T as frame_system::Trait>::AccountId>,
    T::ValidatorIdOf: Convert<
        <T as frame_system::Trait>::AccountId,
        Option<<T as frame_system::Trait>::AccountId>,
    >,
{
    fn disable_validator(validator: &<T as frame_system::Trait>::AccountId) -> Result<bool, ()> {
        <pallet_session::Module<T>>::disable(validator)
    }

    fn validators() -> Vec<<T as frame_system::Trait>::AccountId> {
        <pallet_session::Module<T>>::validators()
    }
}

impl<T: Trait> xpallet_support::traits::Validator<T::AccountId> for Module<T> {
    fn is_validator(who: &T::AccountId) -> bool {
        Self::is_validator(who)
    }

    fn validator_for(name: &[u8]) -> Option<T::AccountId> {
        Self::validator_for(name)
    }
}

impl<T: Trait> Module<T> {
    #[cfg(feature = "std")]
    pub fn initialize_validators(
        validators: &[xp_genesis_builder::ValidatorInfo<T::AccountId, BalanceOf<T>>],
    ) -> DispatchResult {
        for xp_genesis_builder::ValidatorInfo {
            who,
            referral_id,
            self_bonded,
            total_nomination,
            total_weight,
        } in validators
        {
            Self::check_referral_id(referral_id)?;
            if !self_bonded.is_zero() {
                assert!(
                    Self::free_balance(who) >= *self_bonded,
                    "Validator does not have enough balance to bond."
                );
                Self::bond_reserve(who, *self_bonded)?;
                Nominations::<T>::mutate(who, who, |nominator| {
                    nominator.nomination = *self_bonded;
                });
            }
            Self::apply_register(who, referral_id.to_vec());
            // These validators will be chilled on the network startup.
            Self::apply_force_chilled(who);

            ValidatorLedgers::<T>::mutate(who, |validator| {
                validator.total_nomination = *total_nomination;
                validator.last_total_vote_weight = *total_weight;
            });
        }
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn force_bond(
        sender: &T::AccountId,
        target: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if !value.is_zero() {
            Self::bond_reserve(sender, value)?;
            Nominations::<T>::mutate(sender, target, |nominator| {
                nominator.nomination = value;
            });
        }
        Ok(())
    }

    /// Mock the `unbond` operation but lock the funds until the specific height `locked_until`.
    #[cfg(feature = "std")]
    pub fn force_unbond(
        sender: &T::AccountId,
        target: &T::AccountId,
        value: BalanceOf<T>,
        locked_until: T::BlockNumber,
    ) -> DispatchResult {
        // We can not reuse can_unbond() as the target can has no bond but has unbonds.
        // Self::can_unbond(sender, target, value)?;
        ensure!(Self::is_validator(target), Error::<T>::NotValidator);
        ensure!(
            Self::unbonded_chunks_of(sender, target).len()
                < Self::maximum_unbonded_chunk_size() as usize,
            Error::<T>::NoMoreUnbondChunks
        );
        Self::unbond_reserve(sender, value)?;
        Self::mutate_unbonded_chunks(sender, target, value, locked_until);
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn force_set_nominator_vote_weight(
        nominator: &T::AccountId,
        validator: &T::AccountId,
        new_weight: VoteWeight,
    ) {
        Nominations::<T>::mutate(nominator, validator, |nominator| {
            nominator.last_vote_weight = new_weight;
        });
    }

    #[cfg(feature = "std")]
    pub fn force_set_validator_vote_weight(who: &T::AccountId, new_weight: VoteWeight) {
        ValidatorLedgers::<T>::mutate(who, |validator| {
            validator.last_total_vote_weight = new_weight;
        });
    }

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
    fn transfer(from: &T::AccountId, to: &T::AccountId, value: BalanceOf<T>) {
        let _ = T::Currency::transfer(from, to, value, ExistenceRequirement::KeepAlive);
    }

    /// Create/Update a new balance lock on account `who`.
    #[inline]
    fn set_lock(who: &T::AccountId, new_locked: BalanceOf<T>) {
        T::Currency::set_lock(STAKING_ID, who, new_locked, WithdrawReasons::all());
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
        match ForceEra::get() {
            Forcing::ForceAlways | Forcing::ForceNew => (),
            _ => ForceEra::put(Forcing::ForceNew),
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
            validator.last_chilled = Some(<frame_system::Module<T>>::block_number());
        });
    }

    /// Set a lock on `value` of free balance of an account.
    pub(crate) fn bond_reserve(who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        let mut new_locks = Self::locks(who);
        let old_bonded = *new_locks.entry(LockedType::Bonded).or_default();
        let new_bonded = old_bonded + value;

        ensure!(
            Self::free_balance(who) >= new_bonded,
            Error::<T>::InsufficientBalance
        );

        Self::set_lock(who, new_bonded);
        new_locks.insert(LockedType::Bonded, new_bonded);
        Locks::<T>::insert(who, new_locks);

        Ok(())
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
        let current_block = <frame_system::Module<T>>::block_number();

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
        let current_block = <frame_system::Module<T>>::block_number();
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
        Self::bond_reserve(nominator, value)?;
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
            "[apply_unbond] who:{:?}, target: {:?}, value: {:?}",
            who, target, value
        );
        Self::unbond_reserve(who, value)?;

        let locked_until =
            <frame_system::Module<T>>::block_number() + Self::bonding_duration_for(who, target);
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
