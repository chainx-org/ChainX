//! # Staking Module

#![cfg_attr(not(feature = "std"), no_std)]

mod constants;
mod election;
mod impls;
mod reward;
mod rpc;
mod slashing;
mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    storage::IterableStorageMap,
    traits::{Currency, ExistenceRequirement, Get, LockableCurrency, WithdrawReasons},
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{
    traits::{CheckedSub, Convert, SaturatedConversion, Saturating, StaticLookup, Zero},
    DispatchResult,
};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

use chainx_primitives::ReferralId;
use constants::*;
use xp_mining_common::{
    Claim, ComputeMiningWeight, Delta, RewardPotAccountFor, ZeroMiningWeightError,
};
use xp_mining_staking::{AssetMining, SessionIndex, UnbondedIndex};
use xp_runtime::Memo;
use xpallet_support::{debug, traits::TreasuryAccount, RpcBalance};

pub use impls::{IdentificationTuple, SimpleValidatorRewardPotAccountDeterminer};
pub use rpc::*;
pub use types::*;

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

    /// An expected duration of the session.
    ///
    /// This parameter is used to determine the longevity of `heartbeat` transaction
    /// and a rough time when we should start considering sending heartbeats,
    /// since the workers avoids sending them at the very beginning of the session, assuming
    /// there is a chance the authority will produce a block and they won't be necessary.
    type SessionDuration: Get<Self::BlockNumber>;
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

        /// The length of a session in blocks.
        pub BlocksPerSession get(fn blocks_per_session) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(DEFAULT_BLOCKS_PER_SESSION);

        /// The length of a staking era in sessions.
        pub SessionsPerEra get(fn sessions_per_era) config():
            T::BlockNumber = T::BlockNumber::saturated_from::<u64>(12);

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

        /// The map from nominator key to the set of keys of all validators to nominate.
        pub Nominators get(fn nominators):
            map hasher(twox_64_concat) T::AccountId => NominatorProfile<T::BlockNumber>;

        /// The map from validator key to the vote weight ledger of that validator.
        pub ValidatorLedgers get(fn validator_ledgers):
            map hasher(twox_64_concat) T::AccountId => ValidatorLedger<BalanceOf<T>, T::BlockNumber>;

        /// The map from nominator to the vote weight ledger of all nominees.
        pub Nominations get(fn nominations):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) T::AccountId
            => NominatorLedger<BalanceOf<T>, T::BlockNumber>;

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
        config(validators):
            Vec<(T::AccountId, ReferralId, BalanceOf<T>)>;
        config(glob_dist_ratio): (u32, u32);
        config(mining_ratio): (u32, u32);
        build(|config: &GenesisConfig<T>| {
            for &(ref v, ref referral_id, balance) in &config.validators {
                assert!(
                    Module::<T>::free_balance(v) >= balance,
                    "Validator does not have enough balance to bond."
                );
                Module::<T>::check_referral_id(referral_id).expect("Invalid referral_id in genesis");
                Module::<T>::apply_register(v, referral_id.to_vec());
                Module::<T>::apply_bond(v, v, balance).expect("Staking genesis initialization can not fail");
            }
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
        });
    }
}

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as frame_system::Trait>::AccountId
    {
        /// The staker has been rewarded by this amount. `AccountId` is the stash account. [validator, reward_amount]
        Mint(AccountId, Balance),
        /// One validator (and its nominators) has been slashed by the given amount. [validator, slashed_amount]
        Slash(AccountId, Balance),
        /// Nominator has bonded to the validator this amount. [nominator, validator, amount]
        Bond(AccountId, AccountId, Balance),
        /// An account has unbonded this amount. [nominator, validator, amount]
        Unbond(AccountId, AccountId, Balance),
        /// Claim the staking reward. [nominator, validator, dividend]
        Claim(AccountId, AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue. [nominator, amount]
        UnlockUnbondedWithdrawal(AccountId, Balance),
        /// Offenders are forcibly to be chilled due to insufficient reward pot balance. [session_index, chilled_validators]
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
    }
}

impl<T: Trait> From<ZeroMiningWeightError> for Error<T> {
    fn from(_: ZeroMiningWeightError) -> Self {
        Self::ZeroVoteWeight
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Nominate the `target` with `value` of the origin account's balance locked.
        #[weight = 10]
        pub fn bond(origin, target: <T::Lookup as StaticLookup>::Source, #[compact] value: BalanceOf<T>, memo: Memo) {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::NotValidator);
            ensure!(value <= Self::free_balance(&sender), Error::<T>::InsufficientBalance);
            if !Self::is_validator_bonding_itself(&sender, &target) {
                Self::check_validator_acceptable_votes_limit(&target, value)?;
            }

            Self::apply_bond(&sender, &target, value)?;
        }

        /// Move the `value` of current nomination from one validator to another.
        #[weight = 10]
        fn rebond(origin, from: <T::Lookup as StaticLookup>::Source, to: <T::Lookup as StaticLookup>::Source, #[compact] value: BalanceOf<T>, memo: Memo) {
            let sender = ensure_signed(origin)?;
            let from = T::Lookup::lookup(from)?;
            let to = T::Lookup::lookup(to)?;

            memo.check_validity()?;

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
        #[weight = 10]
        fn unbond(origin, target: <T::Lookup as StaticLookup>::Source, #[compact] value: BalanceOf<T>, memo: Memo) {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::NotValidator);
            ensure!(value <= Self::bonded_to(&sender, &target), Error::<T>::InvalidUnbondBalance);
            ensure!(
                Self::unbonded_chunks_of(&sender, &target).len() < Self::maximum_unbonded_chunk_size() as usize,
                Error::<T>::NoMoreUnbondChunks
            );

            Self::apply_unbond(&sender, &target, value)?;
        }

        /// Unlock the frozen unbonded balances that are due.
        #[weight = 10]
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

            // apply withdraw_unbonded
            Self::apply_unlock_unbonded_withdrawal(&sender, value);

            unbonded_chunks.swap_remove(unbonded_index as usize);
            Nominations::<T>::mutate(&sender, &target, |nominator_profile| {
                nominator_profile.unbonded_chunks = unbonded_chunks;
            });

            Self::deposit_event(RawEvent::UnlockUnbondedWithdrawal(sender, value));
        }

        /// Claim the staking reward given the `target` validator.
        #[weight = 10]
        fn claim(origin, target: <T::Lookup as StaticLookup>::Source) {
            let sender = ensure_signed(origin)?;
            let target = T::Lookup::lookup(target)?;

            ensure!(Self::is_validator(&target), Error::<T>::NotValidator);

            <Self as Claim<T::AccountId>>::claim(&sender, &target)?;
        }

        /// Declare the desire to validate for the origin account.
        #[weight = 10]
        fn validate(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::NotValidator);
            Validators::<T>::mutate(sender, |validator_profile| {
                    validator_profile.is_chilled = false;
                }
            );
        }

        /// Declare no desire to validate for the origin account.
        #[weight = 10]
        fn chill(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::NotValidator);
            if Self::is_active(&sender) {
                ensure!(Self::can_force_chilled(), Error::<T>::TooFewActiveValidators);
            }
            Validators::<T>::mutate(sender, |validator_profile| {
                    validator_profile.is_chilled = true;
                    validator_profile.last_chilled = Some(<frame_system::Module<T>>::block_number());
                }
            );
        }

        /// Register to be a validator for the origin account.
        #[weight = 100_000]
        pub fn register(origin, referral_id: ReferralId) {
            let sender = ensure_signed(origin)?;
            Self::check_referral_id(&referral_id)?;
            ensure!(!Self::is_validator(&sender), Error::<T>::AlreadyValidator);
            ensure!(
                (Self::validator_set().count() as u32) < MaximumValidatorCount::get(),
                Error::<T>::TooManyValidators
            );
            Self::apply_register(&sender, referral_id);
        }

        #[weight = 10]
        fn set_validator_count(origin, #[compact] new: u32) {
            ensure_root(origin)?;
            ValidatorCount::put(new);
        }

        #[weight = 10]
        fn set_minimal_validator_count(origin, #[compact] new: u32) {
            ensure_root(origin)?;
            MinimumValidatorCount::put(new);
        }

        #[weight = 10]
        fn set_bonding_duration(origin, #[compact] new: T::BlockNumber) {
            ensure_root(origin)?;
            BondingDuration::<T>::put(new);
        }

        #[weight = 10]
        fn set_validator_bonding_duration(origin, #[compact] new: T::BlockNumber) {
            ensure_root(origin)?;
            ValidatorBondingDuration::<T>::put(new);
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
    fn last_rebond_of(nominator: &T::AccountId) -> Option<T::BlockNumber> {
        Nominators::<T>::get(nominator).last_rebond
    }

    #[inline]
    fn free_balance(who: &T::AccountId) -> BalanceOf<T> {
        T::Currency::free_balance(who)
    }

    /// Returns the total votes of a validator.
    #[inline]
    fn total_votes_of(validator: &T::AccountId) -> BalanceOf<T> {
        ValidatorLedgers::<T>::get(validator).total
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
        const MIMUM_REFERRAL_ID: usize = 2;
        const MAXIMUM_REFERRAL_ID: usize = 12;
        let referral_id_len = referral_id.len();
        ensure!(
            referral_id_len >= MIMUM_REFERRAL_ID && referral_id_len <= MAXIMUM_REFERRAL_ID,
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

    /// Returns true if the number of active validators are more than the minimum validator count.
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

    fn apply_force_chilled(who: &T::AccountId) {
        // Force the validator to be chilled
        Validators::<T>::mutate(who, |validator_profile| {
            validator_profile.is_chilled = true;
            validator_profile.last_chilled = Some(<frame_system::Module<T>>::block_number());
        });
    }

    /// Set a lock on `value` of free balance of an account.
    pub(crate) fn bond_reserve(who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        let mut new_locks = Self::locks(who);
        let old_bonded = *new_locks.entry(LockedType::Bonded).or_default();
        let new_bonded = old_bonded + value;

        Self::free_balance(who)
            .checked_sub(&new_bonded)
            .ok_or(Error::<T>::InsufficientBalance)?;

        Self::set_lock(who, new_bonded);
        new_locks.insert(LockedType::Bonded, new_bonded);
        Locks::<T>::insert(who, new_locks);

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

    /// Settles and update the vote weight state of the nominator `source` and validator `target` given the delta amount.
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
        Self::deposit_event(RawEvent::Bond(nominator.clone(), nominee.clone(), value));
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
        Nominators::<T>::mutate(who, |nominator_profile| {
            nominator_profile.last_rebond = Some(current_block);
        });
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

        let bonding_duration = if Self::is_validator(who) && *who == *target {
            Self::validator_bonding_duration()
        } else {
            Self::bonding_duration()
        };

        let locked_until = <frame_system::Module<T>>::block_number() + bonding_duration;

        let mut unbonded_chunks = Self::unbonded_chunks_of(who, target);

        if let Some(idx) = unbonded_chunks
            .iter()
            .position(|x| x.locked_until == locked_until)
        {
            unbonded_chunks[idx].value += value;
        } else {
            unbonded_chunks.push(Unbonded {
                value,
                locked_until,
            });
        }

        Nominations::<T>::mutate(who, target, |nominator_profile| {
            nominator_profile.unbonded_chunks = unbonded_chunks;
        });

        Self::update_vote_weight(who, target, Delta::Sub(value));

        Self::deposit_event(RawEvent::Unbond(who.clone(), target.clone(), value));

        Ok(())
    }

    fn apply_unlock_unbonded_withdrawal(who: &T::AccountId, value: BalanceOf<T>) {
        let new_bonded = Self::total_locked_of(who) - value;
        Self::set_lock(who, new_bonded);
        Locks::<T>::mutate(who, |locks| {
            let old_value = *locks.entry(LockedType::BondedWithdrawal).or_default();
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
