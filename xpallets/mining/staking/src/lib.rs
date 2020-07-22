//! # Staking Module

#![cfg_attr(not(feature = "std"), no_std)]

mod election;
mod impls;
mod reward;
mod slashing;
mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageMap,
    traits::Get,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{Convert, SaturatedConversion, Saturating, Zero};
use sp_std::prelude::*;

use chainx_primitives::Memo;
use xp_mining_common::{
    Claim, ComputeMiningWeight, Delta, RewardPotAccountFor, ZeroMiningWeightError,
};
use xp_mining_staking::{AssetMining, SessionIndex, TreasuryAccount, UnbondedIndex};
use xpallet_assets::{AssetErr, AssetType};
use xpallet_support::debug;

pub use impls::{IdentificationTuple, SimpleValidatorRewardPotAccountDeterminer};
pub use types::*;

/// Session reward of the first 210_000 sessions.
const INITIAL_REWARD: u64 = 5_000_000_000;
/// Every 210_000 sessions, the session reward is cut in half.
///
/// ChainX follows the issuance rule of Bitcoin. The `Session` in ChainX
/// is equivalent to `Block` in Bitcoin with regard to minting new coins.
const SESSIONS_PER_ROUND: u32 = 210_000;

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const DEFAULT_MAXIMUM_VALIDATOR_COUNT: u32 = 100;
const DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE: u32 = 10;

/// ChainX 2.0 block time is targeted at 6s, i.e., 5 minute per session by default.
const DEFAULT_BLOCKS_PER_SESSION: u64 = 50;
const DEFAULT_BONDING_DURATION: u64 = DEFAULT_BLOCKS_PER_SESSION * 12 * 24 * 3;
const DEFAULT_VALIDATOR_BONDING_DURATION: u64 = DEFAULT_BONDING_DURATION * 10;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

pub trait Trait: xpallet_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    ///
    type TreasuryAccount: TreasuryAccount<Self::AccountId>;

    ///
    type AssetMining: AssetMining<Self::Balance>;

    ///
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
        pub MinimumValidatorCount get(fn minimum_validator_count) config():
            u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;

        /// Maximum number of staking participants before emergency conditions are imposed.
        pub MaximumValidatorCount get(fn maximum_validator_count) config():
            u32 = DEFAULT_MAXIMUM_VALIDATOR_COUNT;

        /// Minimum value (self_bonded, total_bonded) to be a candidate of validator election.
        pub ValidatorCandidateRequirement get(fn validator_bond_requirement):
            BondRequirement<T::Balance>;

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
            map hasher(twox_64_concat) T::AccountId => NominatorProfile<T::Balance, T::BlockNumber>;

        /// The map from validator key to the vote weight ledger of that validator.
        pub ValidatorLedgers get(fn validator_ledgers):
            map hasher(twox_64_concat) T::AccountId => ValidatorLedger<T::Balance, T::BlockNumber>;

        /// The map from nominator to the vote weight ledger of all nominees.
        pub Nominations get(fn nominations):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) T::AccountId
            => NominatorLedger<T::Balance, T::BlockNumber>;

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
        pub MinimumPenalty get(fn minimum_penalty) config(): T::Balance;

        /// The higher the severity, the more slash for the offences.
        pub OffenceSeverity get(fn offence_severity) config(): u32;
    }

    add_extra_genesis {
        config(validators):
            Vec<(T::AccountId, T::Balance)>;
        config(glob_dist_ratio): (u32, u32);
        config(mining_ratio): (u32, u32);
        build(|config: &GenesisConfig<T>| {
            for &(ref v, balance) in &config.validators {
                assert!(
                    <xpallet_assets::Module<T>>::pcx_free_balance(v) >= balance,
                    "Validator does not have enough balance to bond."
                );
                Module::<T>::apply_register(v);
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
        <T as frame_system::Trait>::AccountId,
        <T as xpallet_assets::Trait>::Balance,
    {
        /// The staker has been rewarded by this amount. `AccountId` is the stash account.
        Reward(AccountId, Balance),
        /// One validator (and its nominators) has been slashed by the given amount.
        Slash(AccountId, Balance),
        /// Nominator has bonded to the validator this amount.
        Bond(AccountId, AccountId, Balance),
        /// An account has unbonded this amount.
        Unbond(AccountId, AccountId, Balance),
        ///
        Claim(AccountId, AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue.
        WithdrawUnbonded(AccountId, Balance),
        /// Offenders are forcibly to be chilled due to insufficient reward pot balance.
        ForceChilled(SessionIndex, Vec<AccountId>),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Zero amount
        ZeroBalance,
        ///
        ZeroVoteWeight,
        /// Invalid validator target.
        InvalidValidator,
        /// Can not force validator to be chilled.
        InsufficientActiveValidators,
        /// Free balance can not cover this bond operation.
        InsufficientBalance,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Invalid rebondable value.
        InvalidRebondValue,
        ///
        InvalidUnbondValue,
        /// Can not schedule more unbond chunks.
        NoMoreUnbondChunks,
        /// Validators can not accept more votes from other voters.
        NoMoreAcceptableVotes,
        /// Can not rebond the validator self-bonded.
        ///
        /// Due to the validator and regular nominator have different bonding duration.
        RebondSelfBondedNotAllowed,
        /// Nominator did not nominate that validator before.
        NonexistentNomination,
        ///
        RegisteredAlready,
        ///
        EmptyUnbondedChunk,
        ///
        InvalidUnbondedIndex,
        ///
        UnbondRequestNotYetDue,
        /// Can not rebond due to the restriction of rebond frequency limit.
        NoMoreRebond,
        /// The call is not allowed at the given time due to the restriction of election period.
        CallNotAllowed,
        ///
        AssetError,
    }
}

impl<T: Trait> From<AssetErr> for Error<T> {
    fn from(_: AssetErr) -> Self {
        Self::AssetError
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

        fn on_finalize() {
        }

        /// Nominates the `target` with `value` of the origin account's balance locked.
        #[weight = 10]
        pub fn bond(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::InvalidValidator);
            ensure!(value <= Self::free_balance_of(&sender), Error::<T>::InsufficientBalance);
            if !Self::is_validator_self_bonding(&sender, &target) {
                Self::check_validator_acceptable_votes_limit(&target, value)?;
            }

            Self::apply_bond(&sender, &target, value)?;
        }

        /// Switchs the nomination of `value` from one validator to another.
        #[weight = 10]
        fn rebond(origin, from: T::AccountId, to: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&from) && Self::is_validator(&to), Error::<T>::InvalidValidator);
            ensure!(sender != from, Error::<T>::RebondSelfBondedNotAllowed);

            ensure!(value <= Self::bonded_to(&sender, &from), Error::<T>::InvalidRebondValue);

            if !Self::is_validator_self_bonding(&sender, &to) {
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

        /// Unnominates balance `value` from validator `target`.
        #[weight = 10]
        fn unbond(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::InvalidValidator);
            ensure!(value <= Self::bonded_to(&sender, &target), Error::<T>::InvalidUnbondValue);
            ensure!(
                Self::unbonded_chunks_of(&sender).len() < Self::maximum_unbonded_chunk_size() as usize,
                Error::<T>::NoMoreUnbondChunks
            );

            Self::apply_unbond(&sender, &target, value)?;
        }

        /// Frees the unbonded balances that are due.
        #[weight = 10]
        fn withdraw_unbonded(origin, unbonded_index: UnbondedIndex) {
            let sender = ensure_signed(origin)?;

            let mut unbonded_chunks = Self::unbonded_chunks_of(&sender);
            ensure!(!unbonded_chunks.is_empty(), Error::<T>::EmptyUnbondedChunk);
            ensure!(unbonded_index < unbonded_chunks.len() as u32, Error::<T>::InvalidUnbondedIndex);

            let Unbonded { value, locked_until } = unbonded_chunks[unbonded_index as usize];
            let current_block = <frame_system::Module<T>>::block_number();

            ensure!(current_block > locked_until, Error::<T>::UnbondRequestNotYetDue);

            // apply withdraw_unbonded
            Self::unlock_unbonded_reservation(&sender, value).map_err(|_| Error::<T>::AssetError)?;
            unbonded_chunks.swap_remove(unbonded_index as usize);

            Nominators::<T>::mutate(&sender, |nominator_profile| {
                nominator_profile.unbonded_chunks = unbonded_chunks;
            });

            Self::deposit_event(RawEvent::WithdrawUnbonded(sender, value));
        }

        /// Claims the staking reward given the `target` validator.
        #[weight = 10]
        fn claim(origin, target: T::AccountId) {
            let sender = ensure_signed(origin)?;

            ensure!(Self::is_validator(&target), Error::<T>::InvalidValidator);

            <Self as Claim<T::AccountId>>::claim(&sender, &target)?;
        }

        /// Declare the desire to validate for the origin account.
        #[weight = 10]
        fn validate(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::InvalidValidator);
            Validators::<T>::mutate(sender, |validator_profile| {
                    validator_profile.is_chilled = false;
                }
            );
        }

        /// Declare no desire to validate for the origin account.
        #[weight = 10]
        fn chill(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_validator(&sender), Error::<T>::InvalidValidator);
            if Self::is_active(&sender) {
                ensure!(Self::can_force_chilled(), Error::<T>::InsufficientActiveValidators);
            }
            Validators::<T>::mutate(sender, |validator_profile| {
                    validator_profile.is_chilled = true;
                    validator_profile.last_chilled = Some(<frame_system::Module<T>>::block_number());
                }
            );
        }

        /// TODO: figure out whether this should be kept.
        #[weight = 100_000]
        pub fn register(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(!Self::is_validator(&sender), Error::<T>::RegisteredAlready);
            Self::apply_register(&sender);
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
    pub fn validator_set() -> impl Iterator<Item = T::AccountId> {
        Validators::<T>::iter().map(|(v, _)| v)
    }

    #[inline]
    pub fn active_validator_set() -> impl Iterator<Item = T::AccountId> {
        Self::validator_set().filter(Self::is_active)
    }

    pub fn active_validator_votes() -> impl Iterator<Item = (T::AccountId, T::Balance)> {
        Self::active_validator_set().map(|v| {
            let total_votes = Self::total_votes_of(&v);
            (v, total_votes)
        })
    }

    /// Calculates the total staked PCX, i.e., total staking power.
    ///
    /// One (indivisible) PCX one power, only the votes of active validators are counted.
    #[inline]
    pub fn total_staked() -> T::Balance {
        Self::active_validator_votes().fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + x)
    }

    #[inline]
    pub fn reward_pot_for(validator: &T::AccountId) -> T::AccountId {
        T::DetermineRewardPotAccount::reward_pot_account_for(validator)
    }

    #[inline]
    fn unbonded_chunks_of(nominator: &T::AccountId) -> Vec<Unbonded<T::Balance, T::BlockNumber>> {
        Nominators::<T>::get(nominator).unbonded_chunks
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

    /// Ensures that at the end of the current session there will be a new era.
    fn ensure_new_era() {
        match ForceEra::get() {
            Forcing::ForceAlways | Forcing::ForceNew => (),
            _ => ForceEra::put(Forcing::ForceNew),
        }
    }

    /// Returns true if the number of active validators are more than the minimum validator count.
    fn can_force_chilled() -> bool {
        let mut active_cnt = 0u32;
        let minimum_validator_cnt = Self::minimum_validator_count();
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
            return Err(Error::<T>::InsufficientActiveValidators);
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

    fn total_votes_of(validator: &T::AccountId) -> T::Balance {
        ValidatorLedgers::<T>::get(validator).total
    }

    fn validator_self_bonded(validator: &T::AccountId) -> T::Balance {
        Self::bonded_to(validator, validator)
    }

    #[inline]
    fn bonded_to(nominator: &T::AccountId, nominee: &T::AccountId) -> T::Balance {
        Nominations::<T>::get(nominator, nominee).nomination
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

    fn bond_reserve(who: &T::AccountId, value: T::Balance) -> Result<(), AssetErr> {
        <xpallet_assets::Module<T>>::pcx_move_balance(
            who,
            AssetType::Free,
            who,
            AssetType::ReservedStaking,
            value,
        )
    }

    fn unbond_reserve(who: &T::AccountId, value: T::Balance) -> Result<(), AssetErr> {
        <xpallet_assets::Module<T>>::pcx_move_balance(
            who,
            AssetType::ReservedStaking,
            who,
            AssetType::ReservedStakingRevocation,
            value,
        )
    }

    fn unlock_unbonded_reservation(who: &T::AccountId, value: T::Balance) -> Result<(), AssetErr> {
        <xpallet_assets::Module<T>>::pcx_move_balance(
            who,
            AssetType::ReservedStakingRevocation,
            who,
            AssetType::Free,
            value,
        )
    }

    /// Settles and update the vote weight state of the nominator `source` and validator `target` given the delta amount.
    fn update_vote_weight(source: &T::AccountId, target: &T::AccountId, delta: Delta<T::Balance>) {
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

    fn apply_register(who: &T::AccountId) {
        let current_block = <frame_system::Module<T>>::block_number();
        Validators::<T>::insert(
            who,
            ValidatorProfile {
                registered_at: current_block,
                ..Default::default()
            },
        );
    }

    fn apply_bond(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
        value: T::Balance,
    ) -> Result<(), Error<T>> {
        Self::bond_reserve(nominator, value)?;
        Self::update_vote_weight(nominator, nominee, Delta::Add(value));
        Self::deposit_event(RawEvent::Bond(nominator.clone(), nominee.clone(), value));
        Ok(())
    }

    fn apply_rebond(
        who: &T::AccountId,
        from: &T::AccountId,
        to: &T::AccountId,
        value: T::Balance,
        current_block: T::BlockNumber,
    ) {
        // TODO: reduce one block_number read?
        Self::update_vote_weight(who, from, Delta::Sub(value));
        Self::update_vote_weight(who, to, Delta::Add(value));
        Nominators::<T>::mutate(who, |nominator_profile| {
            nominator_profile.last_rebond = Some(current_block);
        });
    }

    fn apply_unbond(
        who: &T::AccountId,
        target: &T::AccountId,
        value: T::Balance,
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

        let mut unbonded_chunks = Self::unbonded_chunks_of(who);

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

        Nominators::<T>::mutate(who, |nominator_profile| {
            nominator_profile.unbonded_chunks = unbonded_chunks;
        });

        Self::update_vote_weight(who, target, Delta::Sub(value));

        Self::deposit_event(RawEvent::Unbond(who.clone(), target.clone(), value));

        Ok(())
    }
}

impl<T: Trait> Module<T> {
    pub fn validators_info() -> Vec<T::AccountId> {
        Self::validator_set().collect()
    }
}
