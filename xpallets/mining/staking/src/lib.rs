//! # Staking Module

#![cfg_attr(not(feature = "std"), no_std)]

mod impls;
mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use chainx_primitives::AssetId;
use chainx_primitives::Memo;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    storage::IterableStorageMap,
    traits::Get,
    weights::{DispatchInfo, GetDispatchInfo, PostDispatchInfo, Weight},
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{
    Convert, DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SaturatedConversion, Saturating,
    SignedExtension, UniqueSaturatedFrom, UniqueSaturatedInto, Zero,
};
use sp_std::prelude::*;
use types::*;
use xp_staking::{CollectAssetMiningInfo, Delta, OnMinting, UnbondedIndex};
use xpallet_assets::{AssetErr, AssetType};
use xpallet_support::debug;

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const DEFAULT_MAXIMUM_VALIDATOR_COUNT: u32 = 100;
const DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE: u32 = 10;

/// ChainX 2.0 block time is targeted at 6s, i.e., 5 minute per session by default.
const DEFAULT_BLOCKS_PER_SESSION: u64 = 50;
const DEFAULT_BONDING_DURATION: u64 = DEFAULT_BLOCKS_PER_SESSION * 12 * 24 * 3;
const DEFAULT_VALIDATOR_BONDING_DURATION: u64 = DEFAULT_BONDING_DURATION * 10;

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

    add_extra_genesis {
        config(validators):
            Vec<T::AccountId>;
            // Vec<(T::AccountId, T::Balance)>;
        build(|config: &GenesisConfig<T>| {
            // for &(ref v, balance) in &config.validators {
            for v in &config.validators {
                // assert!(
                    // T::Currency::free_balance(&stash) >= balance,
                    // "Stash does not have enough balance to bond."
                // );
            }
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
        InsufficientValidators,
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
        NoUnbondedChunk,
        ///
        InvalidUnbondedIndex,
        ///
        UnbondRequestNotYetDue,
        /// Can not rebond due to the restriction of rebond frequency limit.
        NoMoreRebond,
        /// The call is not allowed at the given time due to restrictions of election period.
        CallNotAllowed,
        ///
        AssetError,
    }
}

impl<T: Trait> From<AssetErr> for Error<T> {
    fn from(asset_err: AssetErr) -> Self {
        Self::AssetError
    }
}

impl<T: Trait> From<xp_staking::ZeroVoteWeightError> for Error<T> {
    fn from(e: xp_staking::ZeroVoteWeightError) -> Self {
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
        fn bond(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::InvalidValidator);
            ensure!(value <= Self::free_balance_of(&sender), Error::<T>::InsufficientBalance);
            if !Self::is_validator_self_bonding(&sender, &target) {
                Self::check_validator_acceptable_votes_limit(&sender, value)?;
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

        ///
        #[weight = 10]
        fn unbond(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
            memo.check_validity()?;

            ensure!(!value.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::is_validator(&target), Error::<T>::InvalidValidator);
            // TODO: is this unneccessary?
            // ensure!(Self::nomination_exists(&sender, &target), Error::<T>::NonexistentNomination);
            ensure!(value <= Self::bonded_to(&sender, &target), Error::<T>::InvalidUnbondValue);
            ensure!(
                Self::unbonded_chunks_of(&sender).len() < Self::maximum_unbonded_chunk_size() as usize,
                Error::<T>::NoMoreUnbondChunks
            );

            Self::apply_unbond(&sender, &target, value)?;
        }

        /// Frees up the unbonded balances that are due.
        #[weight = 10]
        fn withdraw_unbonded(origin, unbonded_index: UnbondedIndex) {
            let sender = ensure_signed(origin)?;

            let mut unbonded_chunks = Self::unbonded_chunks_of(&sender);
            ensure!(!unbonded_chunks.is_empty(), Error::<T>::NoUnbondedChunk);
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
            todo!("ensure nominator record exists");

            <Self as xp_staking::Claim<T::AccountId>>::claim(&sender, &target)?;
        }

        /// Declare the desire to validate for the origin account.
        #[weight = 10]
        fn validate(origin) {
            let sender = ensure_signed(origin)?;
        }

        /// Declare no desire to validate for the origin account.
        #[weight = 10]
        fn chill(origin) {
            let sender = ensure_signed(origin)?;

            // for validator in Validators::<T>::iter(){}
        }

        /// TODO: figure out whether this should be kept.
        #[weight = 100_000]
        fn register(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(!Self::is_validator(&sender), Error::<T>::RegisteredAlready);
            let current_block = <frame_system::Module<T>>::block_number();
            Validators::<T>::insert(sender, ValidatorProfile {
                registered_at: current_block,
                ..Default::default()
            });
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

    fn nomination_exists(nominator: &T::AccountId, nominee: &T::AccountId) -> bool {
        Nominations::<T>::contains_key(nominator, nominee)
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

    // Staking specific assets operation
    //
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
    fn update_vote_weight(source: &T::AccountId, target: &T::AccountId, delta: Delta) {
        let current_block = <frame_system::Module<T>>::block_number();
        let saturated_current_block = current_block.saturated_into::<u32>();

        let source_weight =
            <Self as xp_staking::ComputeVoteWeight<T::AccountId>>::settle_claimer_weight(
                source,
                target,
                saturated_current_block,
            );

        let target_weight =
            <Self as xp_staking::ComputeVoteWeight<T::AccountId>>::settle_claimee_weight(
                target,
                saturated_current_block,
            );

        Self::set_nominator_vote_weight(source, target, source_weight, current_block, delta);
        Self::set_validator_vote_weight(target, target_weight, current_block, delta);
    }

    fn apply_bond(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
        value: T::Balance,
    ) -> Result<(), Error<T>> {
        Self::bond_reserve(nominator, value)?;
        Self::update_vote_weight(
            nominator,
            nominee,
            Delta::Add(value.saturated_into::<u128>()),
        );
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
        let v = value.saturated_into::<u128>();
        // TODO: reduce one block_number read?
        Self::update_vote_weight(who, from, Delta::Sub(v));
        Self::update_vote_weight(who, to, Delta::Add(v));
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

        Self::update_vote_weight(who, target, Delta::Sub(value.saturated_into::<u128>()));

        Self::deposit_event(RawEvent::Unbond(who.clone(), target.clone(), value));

        Ok(())
    }
}
