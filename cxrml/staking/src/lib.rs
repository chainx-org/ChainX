// Copyright 2018 Chainpool.
//! Staking manager: Periodically determines the best set of validators.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;

#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate srml_support as runtime_support;

extern crate sr_std as rstd;

#[macro_use]
extern crate parity_codec_derive;

extern crate parity_codec as codec;
extern crate sr_primitives as primitives;
extern crate srml_balances as balances;
extern crate srml_consensus as consensus;
extern crate srml_session as session;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

extern crate cxrml_support as cxsupport;

#[cfg(test)]
extern crate sr_io as runtime_io;
#[cfg(test)]
extern crate substrate_primitives;

use balances::{address::Address, OnDilution};
use primitives::{
    traits::{As, OnFinalise, One, Zero},
    Perbill,
};
// use rstd::cmp;
use rstd::prelude::*;
use runtime_support::dispatch::Result;
use runtime_support::{Parameter, StorageMap, StorageValue};
use session::OnSessionChange;
use system::ensure_signed;

use cxsupport::storage::btree_map::CodecBTreeMap;

mod mock;
mod tests;

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;

#[derive(PartialEq, Clone)]
#[cfg_attr(test, derive(Debug))]
pub enum LockStatus<BlockNumber: Parameter> {
    Liquid,
    LockedUntil(BlockNumber),
    Bonded,
}

/// Preference of what happens on a slash event.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct ValidatorPrefs<Balance> {
    /// Validator should ensure this many more slashes than is necessary before being unstaked.
    pub unstake_threshold: u32,
    // Reward that validator takes up-front; only the rest is split between themselves and nominators.
    pub validator_payment: Balance,
}

impl<B: Default> Default for ValidatorPrefs<B> {
    fn default() -> Self {
        ValidatorPrefs {
            unstake_threshold: 3,
            validator_payment: Default::default(),
        }
    }
}

/// Statistics of staking
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Stats<AccountId, Balance> {
    pub nominator_count: u64,
    pub candidates: Vec<AccountId>,
    pub total_stake: Balance,
}

impl<B: Default, C: Default> Default for Stats<B, C> {
    fn default() -> Self {
        Stats {
            nominator_count: 0,
            candidates: Default::default(),
            total_stake: Default::default(),
        }
    }
}

/// Profile of intention
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct IntentionProfs<AccountId, Balance, BlockNumber> {
    pub is_active: bool,
    pub url: Vec<u8>,
    pub name: Vec<u8>,
    pub jackpot: Balance,
    pub nominators: Vec<AccountId>,
    pub total_nomination: Balance,
    pub last_total_vote_weight: u64,
    pub last_total_vote_weight_update: BlockNumber,
}

impl<B: Default, C: Default, D: Default> Default for IntentionProfs<B, C, D> {
    fn default() -> Self {
        IntentionProfs {
            is_active: false,
            url: Default::default(),
            name: Default::default(),
            jackpot: Default::default(),
            nominators: Default::default(),
            total_nomination: Default::default(),
            last_total_vote_weight: Default::default(),
            last_total_vote_weight_update: Default::default(),
        }
    }
}

/// Profile of nominator, intention per se is a nominator.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct NominatorProfs<AccountId, Balance> {
    pub total_nomination: Balance,
    pub locked: Balance,
    pub nominees: Vec<AccountId>,
}

impl<B: Default, C: Default> Default for NominatorProfs<B, C> {
    fn default() -> Self {
        NominatorProfs {
            locked: Default::default(),
            nominees: Default::default(),
            total_nomination: Default::default(),
        }
    }
}

/// Profile of nominator, intention per se is a nominator.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct NominationRecord<Balance, BlockNumber> {
    pub nomination: Balance,
    pub last_vote_weight: u64,
    pub last_vote_weight_update: BlockNumber,
}

impl<B: Default, C: Default> Default for NominationRecord<B, C> {
    fn default() -> Self {
        NominationRecord {
            nomination: Default::default(),
            last_vote_weight: Default::default(),
            last_vote_weight_update: Default::default(),
        }
    }
}

pub trait Trait: balances::Trait + session::Trait {
    /// Some tokens minted.
    type OnRewardMinted: OnDilution<<Self as balances::Trait>::Balance>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    #[cfg_attr(feature = "std", serde(bound(deserialize = "T::Balance: ::serde::de::DeserializeOwned")))]
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn stake(origin, name: Vec<u8>, url: Vec<u8>, value: T::Balance) -> Result;
        fn update_stake(origin, value: T::Balance) -> Result;
        fn update_registration(origin, name: Vec<u8>, url: Vec<u8>) -> Result;
        fn deactivate(origin, intentions_index: u32) -> Result;
        fn nominate(origin, target: Address<T::AccountId, T::AccountIndex>, value: T::Balance) -> Result;
        fn unnominate(origin, target_index: u32, value: T::Balance) -> Result;
        fn register_preferences(origin, intentions_index: u32, prefs: ValidatorPrefs<T::Balance>) -> Result;

        fn set_sessions_per_era(new: T::BlockNumber) -> Result;
        fn set_bonding_duration(new: T::BlockNumber) -> Result;
        fn set_validator_count(new: u32) -> Result;
        fn force_new_era(apply_rewards: bool) -> Result;
        fn set_offline_slash_grace(new: u32) -> Result;
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where <T as balances::Trait>::Balance, <T as system::Trait>::AccountId {
        /// All validators have been rewarded by the given balance.
        Reward(Balance),
        /// One validator (and their nominators) has been given a offline-warning (they're still
        /// within their grace). The accrued number of slashes is recorded, too.
        OfflineWarning(AccountId, u32),
        /// One validator (and their nominators) has been slashed by the given amount.
        OfflineSlash(AccountId, Balance),
    }
);

pub type Nominations<T> = CodecBTreeMap<
    <T as system::Trait>::AccountId,
    NominationRecord<<T as balances::Trait>::Balance, <T as system::Trait>::BlockNumber>,
>;

decl_storage! {
    trait Store for Module<T: Trait> as Staking {

        /// The ideal number of staking participants.
        pub ValidatorCount get(validator_count) config(): u32;
        /// Minimum number of staking participants before emergency conditions are imposed.
        pub MinimumValidatorCount get(minimum_validator_count) config(): u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;
        /// The length of a staking era in sessions.
        pub SessionsPerEra get(sessions_per_era) config(): T::BlockNumber = T::BlockNumber::sa(1000);
        /// Reward, per second, that is provided per acceptable session.
        pub RewardPerSec get(reward_per_sec) config(): u64;
        /// Maximum reward, per validator, that is provided per acceptable session.
        pub SessionReward get(session_reward) config(): Perbill = Perbill::from_billionths(60);
        /// Slash, per validator that is taken for the first time they are found to be offline.
        pub OfflineSlash get(offline_slash) config(): Perbill = Perbill::from_millionths(1000); // Perbill::from_fraction() is only for std, so use from_millionths().
        /// Number of instances of offline reports before slashing begins for validators.
        pub OfflineSlashGrace get(offline_slash_grace) config(): u32;
        /// The length of the bonding duration in blocks.
        pub BondingDuration get(bonding_duration) config(): T::BlockNumber = T::BlockNumber::sa(1000);

        /// The current era index.
        pub CurrentEra get(current_era) config(): T::BlockNumber;
        /// Preferences that a validator has.
        pub ValidatorPreferences get(validator_preferences): map T::AccountId => ValidatorPrefs<T::Balance>;
        /// All the accounts with a desire to stake.
        pub Intentions get(intentions) config(): Vec<T::AccountId>;

        /// Maximum reward, per validator, that is provided per acceptable session.
        pub CurrentSessionReward get(current_session_reward) config(): T::Balance;
        /// Slash, per validator that is taken for the first time they are found to be offline.
        pub CurrentOfflineSlash get(current_offline_slash) config(): T::Balance;

        /// The next value of sessions per era.
        pub NextSessionsPerEra get(next_sessions_per_era): Option<T::BlockNumber>;
        /// The session index at which the era length last changed.
        pub LastEraLengthChange get(last_era_length_change): T::BlockNumber;

        /// We are forcing a new era.
        pub ForcingNewEra get(forcing_new_era): Option<()>;

        pub StakeWeight get(stake_weight): map T::AccountId => u64;

        /// All (potential) validator -> reward for each session
        pub SessionRewardOf get(session_reward_of): map T::AccountId => T::Balance;

        /// All intention -> profiles
        pub IntentionProfiles get(intention_profiles): map T::AccountId => IntentionProfs<T::AccountId, T::Balance, T::BlockNumber>;
        /// All nominator -> profiles
        pub NominatorProfiles get(nominator_profiles): map T::AccountId => NominatorProfs<T::AccountId, T::Balance>;
        /// All nominator -> nomination records
        pub NominationRecords get(nominations_of) : map T::AccountId => Nominations<T>;
        /// Whole staking statistics
        pub StakingStats get(staking_stats): Stats<T::AccountId, T::Balance>;

        /// All block number -> accounts waiting to be unlocked at that block
        pub LockedAccountsOf get(locked_accounts_of): map T::BlockNumber => Vec<T::AccountId>;
        /// (nominator, unlock_block) => unlock_value
        pub LockedOf get(locked_of): map (T::AccountId, T::BlockNumber) => T::Balance;
    }

    add_extra_genesis {
        config(intention_profiles): Vec<(T::AccountId, Vec<u8>, Vec<u8>)>;
        build(|storage: &mut primitives::StorageMap, config: &GenesisConfig<T>| {
            use codec::Encode;
            for (acnt, name, url) in config.intention_profiles.iter() {
                let mut iprof: IntentionProfs<T::AccountId, T::Balance, T::BlockNumber> = IntentionProfs::default();
                iprof.name = name.clone();
                iprof.url = url.clone();
                storage.insert(GenesisConfig::<T>::hash(&<IntentionProfiles<T>>::key_for(acnt)).to_vec(), iprof.encode());
            }
        });
    }
}

impl<T: Trait> Module<T> {
    /// Total vote weight of an intention
    pub fn total_vote_weight(intention: &T::AccountId) -> u64 {
        let iprof = <IntentionProfiles<T>>::get(intention);
        iprof.last_total_vote_weight
            + iprof.total_nomination.as_()
                * (<system::Module<T>>::block_number() - iprof.last_total_vote_weight_update).as_()
    }

    /// Total locked balance of a nominator
    pub fn locked(who: &T::AccountId) -> T::Balance {
        <NominatorProfiles<T>>::get(who).locked
    }

    /// Nomination of a nominator to his some nominee
    pub fn nomination_of(nominator: &T::AccountId, nominee: &T::AccountId) -> T::Balance {
        if let Some(record) = <NominationRecords<T>>::get(nominator).0.get(nominee) {
            return record.nomination;
        }
        Zero::zero()
    }

    pub fn nomination_record_of(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
    ) -> NominationRecord<T::Balance, T::BlockNumber> {
        if let Some(record) = <NominationRecords<T>>::get(nominator).0.get(nominee) {
            return record.clone();
        }
        <NominationRecord<T::Balance, T::BlockNumber>>::default()
    }

    pub fn insert_nomination_record(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
        record: NominationRecord<T::Balance, T::BlockNumber>,
    ) {
        let mut nominations = <NominationRecords<T>>::get(nominator);
        nominations.0.insert(nominee.clone(), record);
        <NominationRecords<T>>::insert(nominator, nominations);
    }

    /// All funds a nominator has nominated to his nominees
    pub fn total_nomination_of(nominator: &T::AccountId) -> T::Balance {
        <NominatorProfiles<T>>::get(nominator)
            .nominees
            .into_iter()
            .map(|x| Self::nomination_of(nominator, &x))
            .fold(Zero::zero(), |acc: T::Balance, x| acc + x)
    }

    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }

    // PUBLIC IMMUTABLES

    /// The length of a staking era in blocks.
    pub fn era_length() -> T::BlockNumber {
        Self::sessions_per_era() * <session::Module<T>>::length()
    }

    /// Sum of all validators' total vote weight
    pub fn validators_weight() -> u64 {
        <session::Module<T>>::validators()
            .into_iter()
            .map(|v| Self::total_vote_weight(&v))
            .fold(0, |acc, x| acc + x)
    }

    /// Sum of all candidates' total vote weight
    pub fn candidates_weight() -> u64 {
        <StakingStats<T>>::get()
            .candidates
            .into_iter()
            .map(|v| Self::total_vote_weight(&v))
            .fold(0, |acc, x| acc + x)
    }

    // PUBLIC DISPATCH

    /// Declare the desire to stake for the transactor.
    ///
    /// Effects will be felt at the beginning of the next bra.
    fn stake(origin: T::Origin, name: Vec<u8>, url: Vec<u8>, value: T::Balance) -> Result {
        let who = ensure_signed(origin)?;

        ensure!(
            !<IntentionProfiles<T>>::get(&who).is_active,
            "Cannot stake if already staked."
        );
        ensure!(value.as_() > 0, "Cannot stake non-positive.");
        ensure!(
            value <= <balances::Module<T>>::free_balance(&who),
            "Cannot stake if amount greater than your free balance."
        );

        let mut iprof = <IntentionProfiles<T>>::get(&who);
        iprof.is_active = true;
        iprof.nominators.push(who.clone());

        let mut nprof = <NominatorProfiles<T>>::get(&who);
        nprof.nominees.push(who.clone());

        let mut intentions = <Intentions<T>>::get();
        intentions.push(who.clone());

        let mut stats = <StakingStats<T>>::get();
        stats.nominator_count += 1;

        <IntentionProfiles<T>>::insert(&who, iprof);
        <NominatorProfiles<T>>::insert(&who, nprof);
        <Intentions<T>>::put(intentions);
        <StakingStats<T>>::put(stats);

        Self::apply_update_registration(&who, name, url)?;
        Self::apply_update_stake(&who, value)?;

        Ok(())
    }

    /// Registration renewal
    fn update_registration(origin: T::Origin, name: Vec<u8>, url: Vec<u8>) -> Result {
        let who = ensure_signed(origin)?;

        ensure!(
            <IntentionProfiles<T>>::get(&who).is_active,
            "Cannot update if intention is inactive or has not staked yet."
        );
        ensure!(name.len() <= 32, "Cannot update if name too long.");
        ensure!(url.len() <= 32, "Cannot update if url too long.");

        Self::apply_update_registration(&who, name, url)
    }

    /// Update the existing stake
    fn update_stake(origin: T::Origin, value: T::Balance) -> Result {
        let who = ensure_signed(origin)?;

        ensure!(
            <IntentionProfiles<T>>::get(&who).is_active,
            "Cannot update if intention is inactive or has not staked yet."
        );

        let free_balance = <balances::Module<T>>::free_balance(who.clone());
        let record = Self::nomination_record_of(&who, &who);

        ensure!(value.as_() > 0, "Cannot update non-positive.");
        ensure!(
            value != record.nomination,
            "Cannot update if same as your current stake."
        );
        ensure!(
            value <= free_balance + record.nomination,
            "Cannot update if greater than what you can afford."
        );

        // TODO update stake 0
        let mut iprof = <IntentionProfiles<T>>::get(&who);
        if value.is_zero() {
            if let Some(index) = iprof.nominators.iter().position(|x| *x == who) {
                iprof.nominators.swap_remove(index);
            }
        }
        <IntentionProfiles<T>>::insert(&who, iprof);

        Self::apply_update_stake(&who, value)
    }

    /// Retract the desire to stake for the transactor.
    ///
    /// Effects will be felt at the beginning of the next era.
    fn deactivate(origin: T::Origin, intentions_index: u32) -> Result {
        let who = ensure_signed(origin)?;
        // deactivate fails in degenerate case of having too few existing staked parties
        if Self::intentions().len() <= Self::minimum_validator_count() as usize {
            return Err("cannot deactivate when there are too few staked participants");
        }
        Self::apply_deactivate(&who, intentions_index as usize)
    }

    fn nominate(
        origin: T::Origin,
        target: Address<T::AccountId, T::AccountIndex>,
        value: T::Balance,
    ) -> Result {
        let who = ensure_signed(origin)?;
        let target = <balances::Module<T>>::lookup(target)?;

        ensure!(
            <IntentionProfiles<T>>::get(&target).is_active,
            "Cannot nominate if target is inactive or has not staked yet."
        );
        ensure!(value.as_() > 0, "Cannot stake non-positive.");
        ensure!(
            value <= <balances::Module<T>>::free_balance(&who),
            "Cannot nominate if amount greater than your avaliable free balance"
        );

        // TODO charge fee

        // reserve nominated balance
        <balances::Module<T>>::reserve(&who, value)?;

        let current_block = <system::Module<T>>::block_number();

        // update profile of target intention, i.e., nominee
        let mut iprof = <IntentionProfiles<T>>::get(&target);

        // update stats and nominator per se, i.e., nominator
        let mut nprof = <NominatorProfiles<T>>::get(&who);
        // update (nominator, nominee) => nomination, last_vote_weight, last_era_length_change
        let mut record = Self::nomination_record_of(&who, &target);

        let mut stats = <StakingStats<T>>::get();

        // update nomination record
        record.last_vote_weight +=
            record.nomination.as_() * (current_block - record.last_vote_weight_update).as_();
        record.nomination += value;
        record.last_vote_weight_update = current_block;

        iprof.last_total_vote_weight += iprof.total_nomination.as_()
            * (current_block - iprof.last_total_vote_weight_update).as_();
        iprof.last_total_vote_weight_update = current_block;
        iprof.total_nomination += value;

        nprof.total_nomination += value;

        stats.total_stake += value;
        if nprof.nominees.is_empty() {
            stats.nominator_count += 1;
        }

        // update relationships
        // if nominator nominates nominee for the first time
        if nprof.nominees.iter().find(|&n| n == &target).is_none() {
            nprof.nominees.push(target.clone());
            iprof.nominators.push(who.clone());
        }

        <IntentionProfiles<T>>::insert(&target, iprof);
        <NominatorProfiles<T>>::insert(&who, nprof);
        Self::insert_nomination_record(&who, &target, record);
        <StakingStats<T>>::put(stats);

        Ok(())
    }

    /// Claim reward from target
    fn claim(origin: T::Origin, target_index: u32) -> Result {
        let source = ensure_signed(origin)?;
        let target_index = target_index as usize;

        let nprof = <NominatorProfiles<T>>::get(&source);

        let target = nprof
            .nominees
            .get(target_index)
            .ok_or("Invalid target index")?;

        let current_block = <system::Module<T>>::block_number();

        let mut iprof = <IntentionProfiles<T>>::get(target);
        let mut record = Self::nomination_record_of(&source, &target);

        // latest vote weight for nominator and nominee
        let total_vote_weight = iprof.last_total_vote_weight
            + iprof.total_nomination.as_()
                * (current_block - iprof.last_total_vote_weight_update).as_();

        let vote_weight = record.last_vote_weight
            + record.nomination.as_() * (current_block - record.last_vote_weight_update).as_();

        let dividend = T::Balance::sa(vote_weight * iprof.jackpot.as_() / total_vote_weight);
        <balances::Module<T>>::reward(&source, dividend)?;
        iprof.jackpot -= dividend;

        record.last_vote_weight = 0;
        record.last_vote_weight_update = current_block;

        iprof.last_total_vote_weight = total_vote_weight - vote_weight;
        iprof.last_total_vote_weight_update = current_block;

        <IntentionProfiles<T>>::insert(target.clone(), iprof);
        Self::insert_nomination_record(&source, &target, record);

        Ok(())
    }

    /// Will panic if called when source isn't currently nominating target.
    /// Updates Nominating, NominatorsFor and NominationBalance.
    /// target_index is the index of nominee list, 4 => [3, 2], unnominate 3, target_index = 0
    fn unnominate(origin: T::Origin, target_index: u32, value: T::Balance) -> Result {
        let source = ensure_signed(origin)?;
        let target_index = target_index as usize;

        let nprof = <NominatorProfiles<T>>::get(&source);

        let target = nprof
            .nominees
            .get(target_index)
            .ok_or("Invalid target index")?;

        ensure!(value.as_() > 0, "Cannot unnominate non-positive.");

        let mut record = Self::nomination_record_of(&source, &target);

        let current_nomination = record.nomination;
        ensure!(
            value <= current_nomination,
            "Cannot unnominate if the amount greater than your current nomination."
        );

        // Ok - all valid.

        let mut nprof = <NominatorProfiles<T>>::get(&source);
        let mut iprof = <IntentionProfiles<T>>::get(target);
        let mut stats = <StakingStats<T>>::get();

        let current_block = <system::Module<T>>::block_number();

        // update relationships if withdraw all votes
        if value == current_nomination {
            // update intention profile
            if let Some(index) = iprof.nominators.iter().position(|x| *x == source) {
                iprof.nominators.swap_remove(index);
            }

            // update nominator profile
            nprof.nominees.swap_remove(target_index);

            if nprof.nominees.is_empty() {
                stats.nominator_count -= 1;
            }
        }

        // update nominator profile
        nprof.locked += value;
        nprof.total_nomination -= value;

        // update locked info
        let to_lock = value;
        let lock_until = current_block + Self::bonding_duration();

        Self::lock(&source, to_lock, lock_until);

        // update intention profile
        iprof.last_total_vote_weight += iprof.total_nomination.as_()
            * (current_block - iprof.last_total_vote_weight_update).as_();
        iprof.last_total_vote_weight_update = current_block;
        iprof.total_nomination -= value;

        // update nomination record
        record.last_vote_weight +=
            record.nomination.as_() * (current_block - record.last_vote_weight_update).as_();
        record.last_vote_weight_update = current_block;
        record.nomination -= value;

        <IntentionProfiles<T>>::insert(target, iprof);
        <NominatorProfiles<T>>::insert(&source, nprof);
        Self::insert_nomination_record(&source, &target, record);
        <StakingStats<T>>::put(stats);

        Ok(())
    }

    /// Set the given account's preference for slashing behaviour should they be a validator.
    ///
    /// An error (no-op) if `Self::intentions()[intentions_index] != origin`.
    fn register_preferences(
        origin: T::Origin,
        intentions_index: u32,
        prefs: ValidatorPrefs<T::Balance>,
    ) -> Result {
        let who = ensure_signed(origin)?;

        if Self::intentions().get(intentions_index as usize) != Some(&who) {
            return Err("Invalid index");
        }

        <ValidatorPreferences<T>>::insert(who, prefs);

        Ok(())
    }

    // PRIV DISPATCH

    /// Set the number of sessions in an era.
    fn set_sessions_per_era(new: T::BlockNumber) -> Result {
        <NextSessionsPerEra<T>>::put(&new);
        Ok(())
    }

    /// The length of the bonding duration in eras.
    fn set_bonding_duration(new: T::BlockNumber) -> Result {
        <BondingDuration<T>>::put(&new);
        Ok(())
    }

    /// The length of a staking era in sessions.
    fn set_validator_count(new: u32) -> Result {
        <ValidatorCount<T>>::put(&new);
        Ok(())
    }

    /// Force there to be a new era. This also forces a new session immediately after.
    /// `apply_rewards` should be true for validators to get the session reward.
    fn force_new_era(apply_rewards: bool) -> Result {
        Self::apply_force_new_era(apply_rewards)
    }

    // Just force_new_era without origin check.
    fn apply_force_new_era(apply_rewards: bool) -> Result {
        <ForcingNewEra<T>>::put(());
        <session::Module<T>>::apply_force_new_session(apply_rewards)
    }

    /// Set the offline slash grace period.
    fn set_offline_slash_grace(new: u32) -> Result {
        <OfflineSlashGrace<T>>::put(&new);
        Ok(())
    }

    // PUBLIC MUTABLES (DANGEROUS)

    /// Reward a given (potential) validator by a specific amount. Add the reward to their, and their nominators'
    /// balance, pro-rata.
    fn reward(who: &T::AccountId, reward: T::Balance) {
        let off_the_table = T::Balance::sa(reward.as_() * 2 / 10);
        let _ = <balances::Module<T>>::reward(who, off_the_table);
        let to_jackpot = reward - off_the_table;
        let mut iprof = <IntentionProfiles<T>>::get(who);
        iprof.jackpot += to_jackpot;
        <IntentionProfiles<T>>::insert(who, iprof);
    }

    fn apply_update_registration(who: &T::AccountId, name: Vec<u8>, url: Vec<u8>) -> Result {
        let mut iprof = <IntentionProfiles<T>>::get(who);
        iprof.name = name;
        iprof.url = url;
        <IntentionProfiles<T>>::insert(who, iprof);

        Ok(())
    }

    fn apply_update_stake(who: &T::AccountId, value: T::Balance) -> Result {
        let mut iprof = <IntentionProfiles<T>>::get(who);
        let mut nprof = <NominatorProfiles<T>>::get(who);

        let mut record = Self::nomination_record_of(&who, &who);

        let mut stats = <StakingStats<T>>::get();

        let current_block = <system::Module<T>>::block_number();
        let current_nomination = record.nomination;

        iprof.last_total_vote_weight += iprof.total_nomination.as_()
            * (current_block - iprof.last_total_vote_weight_update).as_();
        iprof.last_total_vote_weight_update = current_block;

        record.last_vote_weight +=
            current_nomination.as_() * (current_block - record.last_vote_weight_update).as_();
        record.last_vote_weight_update = current_block;

        if value < current_nomination {
            // decrease stake
            let to_lock = current_nomination - value;
            let lock_until = current_block + Self::bonding_duration();

            iprof.total_nomination -= to_lock;
            nprof.total_nomination -= to_lock;
            record.nomination -= to_lock;
            stats.total_stake -= to_lock;

            nprof.locked += to_lock;

            // update locked info
            Self::lock(who, to_lock, lock_until);
        } else {
            // increase stake
            let to_reserve = value - current_nomination;
            <balances::Module<T>>::reserve(&who, to_reserve)?;

            iprof.total_nomination += to_reserve;
            nprof.total_nomination += to_reserve;
            record.nomination += to_reserve;
            stats.total_stake += to_reserve;
        }

        <IntentionProfiles<T>>::insert(who.clone(), iprof);
        <NominatorProfiles<T>>::insert(who.clone(), nprof);

        Self::insert_nomination_record(&who, &who, record);

        <StakingStats<T>>::put(stats);

        Ok(())
    }

    /// Actually carry out the deactivate operation.
    /// Assumes `intentions()[intentions_index] == who`.
    fn apply_deactivate(who: &T::AccountId, intentions_index: usize) -> Result {
        let mut intentions = Self::intentions();
        if intentions.get(intentions_index) != Some(who) {
            return Err("Invalid index");
        }

        let current_block = <system::Module<T>>::block_number();
        let mut iprof = <IntentionProfiles<T>>::get(who);

        let mut record = Self::nomination_record_of(&who, &who);

        let mut nprof = <NominatorProfiles<T>>::get(who);
        let mut stats = <StakingStats<T>>::get();

        iprof.is_active = false;

        // update intention profile
        iprof.last_total_vote_weight += iprof.total_nomination.as_()
            * (current_block - iprof.last_total_vote_weight_update).as_();
        iprof.last_total_vote_weight_update = current_block;
        iprof.total_nomination -= record.nomination;

        // TODO optimize?
        if let Some(index) = iprof.nominators.iter().position(|x| x == who) {
            iprof.nominators.swap_remove(index);
        }

        if let Some(index) = nprof.nominees.iter().position(|x| x == who) {
            nprof.nominees.swap_remove(index);
        }
        if nprof.nominees.is_empty() {
            stats.nominator_count -= 1;
        }
        stats.total_stake -= record.nomination;

        // update nomination record
        record.last_vote_weight +=
            record.nomination.as_() * (current_block - record.last_vote_weight_update).as_();
        record.last_vote_weight_update = current_block;
        record.nomination = Zero::zero();

        nprof.total_nomination -= record.nomination;

        let to_lock = record.nomination;
        let lock_until = current_block + Self::bonding_duration();

        Self::lock(&who, to_lock, lock_until);

        <IntentionProfiles<T>>::insert(who, iprof);
        <NominatorProfiles<T>>::insert(who, nprof);
        Self::insert_nomination_record(&who, &who, record);
        <StakingStats<T>>::put(stats);

        intentions.swap_remove(intentions_index);
        <Intentions<T>>::put(intentions);

        <ValidatorPreferences<T>>::remove(who);

        Ok(())
    }

    /// Lock the decreased stake
    fn lock(who: &T::AccountId, to_lock: T::Balance, lock_until: T::BlockNumber) {
        // update the list of all locked accounts at some block
        let mut acnts = <LockedAccountsOf<T>>::get(lock_until);
        if acnts.iter().find(|&a| a == who).is_none() {
            acnts.push(who.clone());
            <LockedAccountsOf<T>>::insert(lock_until, acnts);
        }

        // update all locked balance of a certain account at some block
        let locked = <LockedOf<T>>::get((who.clone(), lock_until));
        <LockedOf<T>>::insert((who.clone(), lock_until), locked + to_lock);
    }

    /// Get the reward for the session, assuming it ends with this block.
    fn this_session_reward(actual_elapsed: T::Moment) -> T::Balance {
        let ideal_elapsed = <session::Module<T>>::ideal_session_duration();
        if ideal_elapsed.is_zero() {
            return Self::current_session_reward();
        }
        let per65536: u64 = (T::Moment::sa(65536u64) * ideal_elapsed.clone()
            / actual_elapsed.max(ideal_elapsed))
        .as_();
        Self::current_session_reward() * T::Balance::sa(per65536) / T::Balance::sa(65536u64)
    }

    /// Unlock all matured locked_accounts
    fn unlock_matured_reservation() {
        let block_number = <system::Module<T>>::block_number();
        let locked_accounts = <LockedAccountsOf<T>>::get(block_number);

        for account in locked_accounts.into_iter() {
            let to_unlock = <LockedOf<T>>::take((account.clone(), block_number));
            <balances::Module<T>>::unreserve(&account, to_unlock);
            let mut nprof = <NominatorProfiles<T>>::get(&account);
            nprof.locked -= to_unlock;
            <NominatorProfiles<T>>::insert(&account, nprof);
        }

        <LockedAccountsOf<T>>::remove(block_number);
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session(_actual_elapsed: T::Moment, should_reward: bool) {
        Self::unlock_matured_reservation();

        if should_reward {
            // apply good session reward
            // let reward = Self::this_session_reward(actual_elapsed);
            let session_length: u64 = <session::Module<T>>::length().as_();
            let block_period: u64 = <timestamp::Module<T>>::block_period().as_();
            let reward = T::Balance::sa(session_length * block_period * <RewardPerSec<T>>::get());

            let vals_weight = Self::validators_weight();
            let cands_weight = Self::candidates_weight();
            let total_weight = vals_weight + cands_weight;

            // TODO Genesis vote weight is 0
            let vals_reward = match total_weight {
                0 => 0,
                _ => vals_weight * reward.as_() / total_weight,
            };
            let cands_reward = match total_weight {
                0 => 0,
                _ => cands_weight * reward.as_() / total_weight,
            };

            let mut total_minted: T::Balance = Zero::zero();
            //// reward validators
            let validators = <session::Module<T>>::validators();
            if vals_reward > 0 {
                for v in validators.iter() {
                    let val_reward =
                        T::Balance::sa(Self::total_vote_weight(v) * vals_reward / vals_weight);
                    <SessionRewardOf<T>>::insert(v, val_reward);
                    total_minted += val_reward;
                    Self::reward(v, val_reward);
                }
            }

            //// reward candidates
            let candidates = <StakingStats<T>>::get().candidates;
            if cands_reward > 0 {
                for c in candidates.iter() {
                    let cand_reward =
                        T::Balance::sa(Self::total_vote_weight(c) * cands_reward / cands_weight);
                    <SessionRewardOf<T>>::insert(c, cand_reward);
                    total_minted += cand_reward;
                    Self::reward(c, cand_reward);
                }
            }

            Self::deposit_event(RawEvent::Reward(reward));
            //// TODO Self::total_stake?
            let stats = <StakingStats<T>>::get();
            T::OnRewardMinted::on_dilution(total_minted, stats.total_stake);
        }

        let session_index = <session::Module<T>>::current_index();
        if <ForcingNewEra<T>>::take().is_some()
            || ((session_index - Self::last_era_length_change()) % Self::sessions_per_era())
                .is_zero()
        {
            Self::new_era();
        }
    }

    /// The era has changed - enact new staking set.
    ///
    /// NOTE: This always happens immediately before a session change to ensure that new validators
    /// get a chance to set their session keys.
    fn new_era() {
        // Increment current era.
        <CurrentEra<T>>::put(&(<CurrentEra<T>>::get() + One::one()));

        // Enact era length change.
        if let Some(next_spe) = Self::next_sessions_per_era() {
            if next_spe != Self::sessions_per_era() {
                <SessionsPerEra<T>>::put(&next_spe);
                <LastEraLengthChange<T>>::put(&<session::Module<T>>::current_index());
            }
        }

        // evaluate desired staking amounts and nominations and optimise to find the best
        // combination of validators, then use session::internal::set_validators().
        // for now, this just orders would-be stakers by their balances and chooses the top-most
        // <ValidatorCount<T>>::get() of them.
        // TODO: this is not sound. this should be moved to an off-chain solution mechanism.
        let mut intentions = Self::intentions()
            .into_iter()
            .map(|v| (Self::total_vote_weight(&v), v))
            .collect::<Vec<_>>();

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators
        if intentions.len() < Self::minimum_validator_count() as usize {
            return;
        }

        intentions.sort_unstable_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));

        for (total_vote_weight, intention) in intentions.iter() {
            <StakeWeight<T>>::insert(intention, *total_vote_weight);
        }

        let desired_validator_count = <ValidatorCount<T>>::get() as usize;

        let vals = &intentions
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .take(desired_validator_count)
            .collect::<Vec<_>>();

        <session::Module<T>>::set_validators(vals);

        // All intentions become candidates except the top M validatos.
        let vals_and_candidates = &intentions
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>();

        let mut candidates: Vec<T::AccountId> = Vec::new();
        if vals_and_candidates.len() > vals.len() {
            let start = vals.len();
            for i in start..vals_and_candidates.len() {
                candidates.push(vals_and_candidates[i].clone());
            }
        }
        let mut stats = <StakingStats<T>>::get();
        stats.candidates = candidates;
        <StakingStats<T>>::put(stats);

        // Update the balances for slashing/rewarding according to the stakes.
        // <CurrentOfflineSlash<T>>::put(Self::offline_slash().times(average_stake));
        // <CurrentSessionReward<T>>::put(Self::session_reward().times(average_stake));

        // Disable slash mechanism
        <CurrentOfflineSlash<T>>::put(T::Balance::sa(0u64));
        let session_length: u64 = <session::Module<T>>::length().as_();
        let block_period: u64 = <timestamp::Module<T>>::block_period().as_();
        <CurrentSessionReward<T>>::put(T::Balance::sa(
            session_length * block_period * <RewardPerSec<T>>::get(),
        ));
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_n: T::BlockNumber) {}
}

impl<T: Trait> OnSessionChange<T::Moment> for Module<T> {
    fn on_session_change(elapsed: T::Moment, should_reward: bool) {
        Self::new_session(elapsed, should_reward);
    }
}

impl<T: Trait> balances::EnsureAccountLiquid<T::AccountId> for Module<T> {
    fn ensure_account_liquid(who: &T::AccountId) -> Result {
        if !<balances::Module<T>>::free_balance(who).is_zero() {
            Ok(())
        } else {
            Err("cannot transfer illiquid funds")
        }
    }
}

impl<T: Trait> balances::OnFreeBalanceZero<T::AccountId> for Module<T> {
    fn on_free_balance_zero(_who: &T::AccountId) {}
}

impl<T: Trait> consensus::OnOfflineValidator for Module<T> {
    fn on_offline_validator(_validator_index: usize) {}
}
