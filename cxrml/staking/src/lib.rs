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

#[cfg(test)]
extern crate substrate_primitives;
#[cfg(test)]
extern crate sr_io as runtime_io;

use rstd::prelude::*;
use rstd::cmp;
use runtime_support::{Parameter, StorageValue, StorageMap};
use runtime_support::dispatch::Result;
use session::OnSessionChange;
use primitives::{Perbill, traits::{Zero, One, Bounded, OnFinalise, As}};
use balances::{address::Address, OnDilution};
use system::ensure_signed;

mod mock;
mod tests;


const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const MAX_INTENTIONS: u32 = 100;

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

pub trait Trait: balances::Trait + session::Trait {
    /// Some tokens minted.
    type OnRewardMinted: OnDilution<<Self as balances::Trait>::Balance>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    #[cfg_attr(feature = "std", serde(bound(deserialize = "T::Balance: ::serde::de::DeserializeOwned")))]
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn stake(origin, name: Vec<u8>, url: Vec<u8>) -> Result;
        fn unstake(origin, intentions_index: u32) -> Result;
        fn nominate(origin, target: Address<T::AccountId, T::AccountIndex>, value: T::Balance) -> Result;
        fn unnominate(origin, target_index: u32) -> Result;
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

pub type PairOf<T> = (T, T);

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
        /// All nominator -> nominee relationships.
        pub Nominating get(nominating): map T::AccountId => Option<T::AccountId>;
        /// Nominators for a particular account.
        pub NominatorsFor get(nominators_for): map T::AccountId => Vec<T::AccountId>;
        /// Nominators for a particular account that is in action right now.
        pub CurrentNominatorsFor get(current_nominators_for): map T::AccountId => Vec<T::AccountId>;

        pub StakeWeight get(stake_weight): map T::AccountId => T::Balance;

        /// Maximum reward, per validator, that is provided per acceptable session.
        pub CurrentSessionReward get(current_session_reward) config(): T::Balance;
        /// Slash, per validator that is taken for the first time they are found to be offline.
        pub CurrentOfflineSlash get(current_offline_slash) config(): T::Balance;

        /// The next value of sessions per era.
        pub NextSessionsPerEra get(next_sessions_per_era): Option<T::BlockNumber>;
        /// The session index at which the era length last changed.
        pub LastEraLengthChange get(last_era_length_change): T::BlockNumber;

        /// The highest and lowest staked validator slashable balances.
        pub StakeRange get(stake_range): PairOf<T::Balance>;

        /// The total stake.
        pub TotalStake get(total_stake): T::Balance;

        /// The block at which the `who`'s funds become entirely liquid.
        pub Bondage get(bondage): map T::AccountId => T::BlockNumber;
        /// The number of times a given validator has been reported offline. This gets decremented by one each era that passes.
        pub SlashCount get(slash_count): map T::AccountId => u32;

        /// We are forcing a new era.
        pub ForcingNewEra get(forcing_new_era): Option<()>;

        /// All nominator -> nominees
        pub NomineesOf get(nominees_of): map T::AccountId => Vec<T::AccountId>;

        /// nominations by validator himself
        pub NominationOfValidatorPerSe get(nomination_of_validator_per_se): map T::AccountId => T::Balance;
        /// nominations of nominators for a particular validator
        pub NominationsForValidator get(nominations_for_validator): map T::AccountId => T::Balance;
        /// All nominator -> all funds a nominator has nominated
        pub NominationsOf get(nominations_of): map T::AccountId => T::Balance;

        pub NameOfIntention get(name_of_intention): map T::AccountId => Vec<u8>;
        pub UrlOfIntention get(url_of_intention): map T::AccountId => Vec<u8>;

        /// (nominator, nominee) => value
        pub NominationTo get(nomination_to): map (T::AccountId, T::AccountId) => T::Balance;
        /// (nominator, unlock_block) => unlock_value
        pub LockedOf get(locked_of): map (T::AccountId, T::BlockNumber) => T::Balance;

        /// All block number -> accounts waiting to be unlocked at that block
        pub LockedAccountsOf get(locked_accounts_of): map T::BlockNumber => Vec<T::AccountId>;

        /// The number of accounts who has non-zero nominations, including these who have staked for they nominate themselves actually.
        pub NominatorCount get(nominator_count): u32;

        /// The current set of candidates.
        pub Candidates get(candidates): Vec<T::AccountId>;
        /// The ideal number of staking runner-ups. candidates : validators = 4:1
        pub CandidateCount get(candidate_count) config(): u32;
        /// All (potential) validator -> reward for each session
        pub SessionRewardOf get(session_reward_of): map T::AccountId => T::Balance;
    }

    add_extra_genesis {
        config(name_of_intention): Vec<(T::AccountId, Vec<u8>)>;
        config(url_of_intention): Vec<(T::AccountId, Vec<u8>)>;
        build(|storage: &mut primitives::StorageMap, config: &GenesisConfig<T>| {
            use codec::Encode;
            for (acnt, name) in config.name_of_intention.iter() {
                storage.insert(GenesisConfig::<T>::hash(&<NameOfIntention<T>>::key_for(acnt)).to_vec(), name.encode());
            }
            for (acnt, url) in config.url_of_intention.iter() {
                storage.insert(GenesisConfig::<T>::hash(&<UrlOfIntention<T>>::key_for(acnt)).to_vec(), url.encode());
            }
            // TODO
            // for intention in config.intentions.iter() {
                // storage.insert(GenesisConfig::<T>::hash(&<NominationOfValidatorPerSe<T>>::key_for(intention)).to_vec(), <balances::Module<T>>::total_balance(intention)::encode());
            // }
        });
    }
}

impl<T: Trait> Module<T> {

    /// Nomination of a nominator to his some nominee
    pub fn nomination_of(nominator: &T::AccountId, nominee: &T::AccountId) -> T::Balance {
        <NominationTo<T>>::get((nominator.clone(), nominee.clone()))
    }

    /// All funds a nominator has nominated to his nominees
    pub fn total_nomination_of(nominator: &T::AccountId) -> T::Balance {
        Self::nominees_of(nominator).into_iter()
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

    /// Balance of a (potential) validator that only includes all nominators.
    pub fn nomination_balance(who: &T::AccountId) -> T::Balance {
        Self::nominators_for(who).into_iter()
            .map(|x| Self::nomination_of(&x, who))
            .fold(Zero::zero(), |acc: T::Balance, x| acc + x)
    }

    /// The total balance that can be slashed from an account.
    pub fn slashable_balance(who: &T::AccountId) -> T::Balance {
        Self::nominators_for(who).into_iter()
            .map(|x| Self::nomination_of(&x, who))
            .fold(<balances::Module<T>>::total_balance(who), |acc: T::Balance, x| acc + x)
    }

    pub fn validators_weight() -> T::Balance {
        <session::Module<T>>::validators().into_iter()
            .map(|v| Self::slashable_balance(&v))
            .fold(Zero::zero(), |acc: T::Balance, x| acc + x)
    }

    pub fn candidates_weight() -> T::Balance {
        <Candidates<T>>::get().into_iter()
            .map(|v| Self::slashable_balance(&v))
            .fold(Zero::zero(), |acc: T::Balance, x| acc + x)
    }

    /// The block at which the `who`'s funds become entirely liquid.
    pub fn unlock_block(who: &T::AccountId) -> LockStatus<T::BlockNumber> {
        match Self::bondage(who) {
            i if i == T::BlockNumber::max_value() => LockStatus::Bonded,
            i if i <= <system::Module<T>>::block_number() => LockStatus::Liquid,
            i => LockStatus::LockedUntil(i),
        }
    }

    // PUBLIC DISPATCH

    /// Declare the desire to stake for the transactor.
    ///
    /// Effects will be felt at the beginning of the next era.
    fn stake(origin: T::Origin, name: Vec<u8>, url: Vec<u8>) -> Result {
        let who = ensure_signed(origin)?;

        ensure!(name.len() <= 32, "name too long");
        ensure!(url.len() <= 32, "url too long");

        ensure!(Self::nominating(&who).is_none(), "Cannot stake if already nominating.");
        let mut intentions = <Intentions<T>>::get();
        // can't be in the list twice.
        ensure!(intentions.iter().find(|&t| t == &who).is_none(), "Cannot stake if already staked.");

        <NameOfIntention<T>>::insert(&who, name);
        <UrlOfIntention<T>>::insert(&who, url);

        <Bondage<T>>::insert(&who, T::BlockNumber::max_value());
        intentions.push(who);
        <Intentions<T>>::put(intentions);
        <NominatorCount<T>>::put(<NominatorCount<T>>::get() + 1);
        Ok(())
    }

    /// Retract the desire to stake for the transactor.
    ///
    /// Effects will be felt at the beginning of the next era.
    fn unstake(origin: T::Origin, intentions_index: u32) -> Result {
        let who = ensure_signed(origin)?;
        // unstake fails in degenerate case of having too few existing staked parties
        if Self::intentions().len() <= Self::minimum_validator_count() as usize {
            return Err("cannot unstake when there are too few staked participants")
        }
        Self::apply_unstake(&who, intentions_index as usize)
    }

    fn nominate(origin: T::Origin, target: Address<T::AccountId, T::AccountIndex>, value: T::Balance) -> Result {
        let who = ensure_signed(origin)?;
        let target = <balances::Module<T>>::lookup(target)?;

        ensure!(Self::intentions().into_iter().any(|t| t == target), "Cannot nominate if target is outside the intentions list.");
        ensure!(<balances::Module<T>>::free_balance(&who) >= value, "Cannot nominate if free balance too low.");
        ensure!(!value.is_zero(), "Cannot nominate zero.");

        // reserve nominated balance
        <balances::Module<T>>::reserve(&who, value)?;

        // update votes of who => target
        let v = <NominationTo<T>>::get((who.clone(), target.clone()));
        <NominationTo<T>>::insert((who.clone(), target.clone()), v + value);

        // update nominators_for
        let mut noms = Self::nominators_for(&target);
        if noms.iter().find(|&n| n == &who).is_none() {
            noms.push(who.clone());
            <NominatorsFor<T>>::insert(&target, noms);
        }

        // update nominatings_of
        let mut ns = Self::nominees_of(&who);
        if ns.is_empty() {
            <NominatorCount<T>>::put(<NominatorCount<T>>::get() + 1);
        }
        if ns.iter().find(|&n| n == &target).is_none() {
            ns.push(target.clone());
            <NomineesOf<T>>::insert(&who, ns);
        }

        // update nominating
        // Now this indicates the last nominee of the nominator
        <Nominating<T>>::insert(&who, &target);

        <NominationsOf<T>>::insert(&who, Self::total_nomination_of(&who));
        <NominationsForValidator<T>>::insert(&target, Self::nomination_balance(&target));

        Ok(())
    }

    /// Will panic if called when source isn't currently nominating target.
    /// Updates Nominating, NominatorsFor and NominationBalance.
    /// target_index is the index of nominee list, 4 => [3, 2], unnominate 3, target_index = 0
    fn unnominate(origin: T::Origin, target_index: u32) -> Result {
        let source = ensure_signed(origin)?;
        let target_index = target_index as usize;

        let ns = Self::nominees_of(&source);
        let target = ns.get(target_index).ok_or("Invalid target index")?;

        // Ok - all valid.

        // update nominators_for
        let mut noms = Self::nominators_for(target);
        if let Some(index) = noms.iter().position(|x| *x == source) {
            noms.swap_remove(index);
            <NominatorsFor<T>>::insert(target, noms);
        }

        // update nominees_of
        let mut ns = Self::nominees_of(&source);
        ns.swap_remove(target_index);
        <NomineesOf<T>>::insert(&source, ns);

        // update last nominating relationship
        if Self::nominees_of(&source).is_empty() {
            <Nominating<T>>::remove(&source);
            <NominatorCount<T>>::put(<NominatorCount<T>>::get() - 1);
        }

        // update nominee: [nominator : value]
        let locked = <NominationTo<T>>::take((source.clone(), target.clone()));
        let lock_until = <system::Module<T>>::block_number() + Self::bonding_duration();

        // update all locked accounts at some block
        let mut acnts = <LockedAccountsOf<T>>::get(lock_until);
        if acnts.iter().find(|&a| a == &source).is_none() {
            acnts.push(source.clone());
            <LockedAccountsOf<T>>::insert(lock_until, acnts);
        }

        // update all locked balance of a certain account at some block
        // a nominator could nominate/unnominate multiple nominees in a block at the same time.
        let l = <LockedOf<T>>::get((source.clone(), lock_until));
        <LockedOf<T>>::insert((source.clone(), lock_until), l + locked);

        <NominationsOf<T>>::insert(&source, Self::total_nomination_of(&source));
        <NominationsForValidator<T>>::insert(target, Self::nomination_balance(target));

        Ok(())
    }

    /// Set the given account's preference for slashing behaviour should they be a validator.
    ///
    /// An error (no-op) if `Self::intentions()[intentions_index] != origin`.
    fn register_preferences(
        origin: T::Origin,
        intentions_index: u32,
        prefs: ValidatorPrefs<T::Balance>
    ) -> Result {
        let who = ensure_signed(origin)?;

        if Self::intentions().get(intentions_index as usize) != Some(&who) {
            return Err("Invalid index")
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

    /// Slash a given validator by a specific amount. Removes the slash from their balance by preference,
    /// and reduces the nominators' balance if needed.
    fn slash_validator(v: &T::AccountId, slash: T::Balance) {
        // skip the slash in degenerate case of having only 4 staking participants despite having a larger
        // desired number of validators (validator_count).
        if Self::intentions().len() <= Self::minimum_validator_count() as usize {
            return
        }

        if let Some(rem) = <balances::Module<T>>::slash(v, slash) {
            let noms = Self::current_nominators_for(v);
            let total = noms.iter().map(<balances::Module<T>>::total_balance).fold(T::Balance::zero(), |acc, x| acc + x);
            if !total.is_zero() {
                let safe_mul_rational = |b| b * rem / total;// TODO: avoid overflow
                for n in noms.iter() {
                    let _ = <balances::Module<T>>::slash(n, safe_mul_rational(<balances::Module<T>>::total_balance(n)));	// best effort - not much that can be done on fail.
                }
            }
        }
    }

    /// Reward a given (potential) validator by a specific amount. Add the reward to their, and their nominators'
    /// balance, pro-rata.
    fn reward(who: &T::AccountId, reward: T::Balance) {
        // let off_the_table = reward.min(Self::validator_preferences(who).validator_payment);
        let off_the_table = T::Balance::sa(reward.as_() * 2 / 10);
        let reward = reward - off_the_table;
        let validator_cut = if reward.is_zero() {
            Zero::zero()
        } else {
            let total = Self::nomination_balance(who) + <balances::Module<T>>::total_balance(who);

            let safe_mul_rational = |b| b * reward / total;// TODO: avoid overflow

            let noms = Self::nominators_for(who);
            for nom in noms.iter() {
                let _ = <balances::Module<T>>::reward(nom, safe_mul_rational(Self::nomination_of(nom, who)));
            }
            safe_mul_rational(<balances::Module<T>>::total_balance(who))
        };
        let _ = <balances::Module<T>>::reward(who, validator_cut + off_the_table);
    }

    /// Actually carry out the unstake operation.
    /// Assumes `intentions()[intentions_index] == who`.
    fn apply_unstake(who: &T::AccountId, intentions_index: usize) -> Result {
        let mut intentions = Self::intentions();
        if intentions.get(intentions_index) != Some(who) {
            return Err("Invalid index");
        }
        intentions.swap_remove(intentions_index);
        <Intentions<T>>::put(intentions);
        <ValidatorPreferences<T>>::remove(who);
        <SlashCount<T>>::remove(who);
        <Bondage<T>>::insert(who, <system::Module<T>>::block_number() + Self::bonding_duration());

        <NominationOfValidatorPerSe<T>>::remove(who);

        <NameOfIntention<T>>::remove(who);
        <UrlOfIntention<T>>::remove(who);

        <NominatorCount<T>>::put(<NominatorCount<T>>::get() - 1);
        Ok(())
    }

    /// Get the reward for the session, assuming it ends with this block.
    fn this_session_reward(actual_elapsed: T::Moment) -> T::Balance {
        let ideal_elapsed = <session::Module<T>>::ideal_session_duration();
        if ideal_elapsed.is_zero() {
            return Self::current_session_reward();
        }
        let per65536: u64 = (T::Moment::sa(65536u64) * ideal_elapsed.clone() / actual_elapsed.max(ideal_elapsed)).as_();
        Self::current_session_reward() * T::Balance::sa(per65536) / T::Balance::sa(65536u64)
    }

    /// Unlock all matured locked_accounts
    fn unlock_matured_reservation() {
        let block_number = <system::Module<T>>::block_number();
        let locked_accounts = <LockedAccountsOf<T>>::get(block_number);

        for account in locked_accounts.into_iter() {
            let locked = <LockedOf<T>>::take((account.clone(), block_number));
            <balances::Module<T>>::unreserve(&account, locked);
        }

        <LockedAccountsOf<T>>::remove(block_number);
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session(actual_elapsed: T::Moment, should_reward: bool) {

        Self::unlock_matured_reservation();

        if should_reward {
            // apply good session reward
            let reward = Self::this_session_reward(actual_elapsed);

            let vals_weight = Self::validators_weight().as_();
            let cands_weight = Self::candidates_weight().as_();
            let valid_cands_weight = cands_weight * 7 / 10;
            let total_weight = vals_weight + valid_cands_weight;

            let vals_reward = vals_weight * reward.as_() / total_weight;
            let cands_reward = valid_cands_weight * reward.as_() / total_weight;

            //// reward validators
            let validators = <session::Module<T>>::validators();
            if vals_reward > 0 {
                for v in validators.iter() {
                    let val_reward = Self::slashable_balance(v).as_() * vals_reward / vals_weight;
                    <SessionRewardOf<T>>::insert(v, T::Balance::sa(val_reward));
                    Self::reward(v, T::Balance::sa(val_reward));

                    <NominationOfValidatorPerSe<T>>::insert(v, <balances::Module<T>>::total_balance(v));
                }
            }

            //// reward candidates
            let candidates = <Candidates<T>>::get();
            if cands_reward > 0 {
                for c in candidates.iter() {
                    let cand_reward = Self::slashable_balance(c).as_() * cands_reward / cands_weight;
                    <SessionRewardOf<T>>::insert(c, T::Balance::sa(cand_reward));
                    Self::reward(c, T::Balance::sa(cand_reward));
                }
            }

            Self::deposit_event(RawEvent::Reward(reward));
            let total_minted = reward * <T::Balance as As<usize>>::sa(validators.len());
            T::OnRewardMinted::on_dilution(total_minted, Self::total_stake());
        }

        let session_index = <session::Module<T>>::current_index();
        if <ForcingNewEra<T>>::take().is_some()
            || ((session_index - Self::last_era_length_change()) % Self::sessions_per_era()).is_zero()
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
            .map(|v| (Self::slashable_balance(&v), v))
            .collect::<Vec<_>>();

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators
        if intentions.len() < Self::minimum_validator_count() as usize {
            return
        }

        intentions.sort_unstable_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));

        let desired_validator_count = <ValidatorCount<T>>::get() as usize;
        let n = cmp::min(intentions.len() as u32, MAX_INTENTIONS) as usize;
        let stake_range = if !intentions.is_empty() {
            (intentions[0].0, intentions[n - 1].0)
        } else {
            (Zero::zero(), Zero::zero())
        };
        <StakeRange<T>>::put(&stake_range);

        for (slashable, intention) in intentions.iter() {
            <StakeWeight<T>>::insert(intention, slashable);
        }

        let vals = &intentions.clone().into_iter()
            .map(|(_, v)| v)
            .take(desired_validator_count)
            .collect::<Vec<_>>();
        for v in <session::Module<T>>::validators().iter() {
            <CurrentNominatorsFor<T>>::remove(v);
            let slash_count = <SlashCount<T>>::take(v);
            if slash_count > 1 {
                <SlashCount<T>>::insert(v, slash_count - 1);
            }
        }
        <session::Module<T>>::set_validators(vals);

        let candidate_count = <CandidateCount<T>>::get() as usize;
        let vals_and_candidates = &intentions.clone().into_iter()
            .map(|(_, v)| v)
            .take(desired_validator_count + candidate_count)
            .collect::<Vec<_>>();

        let mut candidates: Vec<T::AccountId> = Vec::new();
        if vals_and_candidates.len() > vals.len() {
            let start = vals.len();
            for i in start..vals_and_candidates.len() {
                candidates.push(vals_and_candidates[i].clone());
            }
        }
        <Candidates<T>>::put(candidates);

        let vals = &intentions.into_iter()
            .map(|(_, v)| v)
            .take(n)
            .collect::<Vec<_>>();
        for v in vals.iter() {
            <CurrentNominatorsFor<T>>::insert(v, Self::nominators_for(v));
        }
        let total_stake: T::Balance = Self::intentions()
            .into_iter()
            .take(n)
            .map(|v| Self::slashable_balance(&v))
            .fold(Zero::zero(), |acc, x| acc + x);

        <TotalStake<T>>::put(&total_stake);
        let _average_stake = total_stake / T::Balance::sa(n as u64);

        // Update the balances for slashing/rewarding according to the stakes.
        // <CurrentOfflineSlash<T>>::put(Self::offline_slash().times(average_stake));
        // <CurrentSessionReward<T>>::put(Self::session_reward().times(average_stake));

        // Disable slash mechanism
        <CurrentOfflineSlash<T>>::put(T::Balance::sa(0u64));
        let session_length: u64 = <session::Module<T>>::length().as_();
        let block_period: u64 = <timestamp::Module<T>>::block_period().as_();
        <CurrentSessionReward<T>>::put(T::Balance::sa(session_length * block_period * <RewardPerSec<T>>::get()));
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_n: T::BlockNumber) {
    }
}

impl<T: Trait> OnSessionChange<T::Moment> for Module<T> {
    fn on_session_change(elapsed: T::Moment, should_reward: bool) {
        Self::new_session(elapsed, should_reward);
    }
}

impl<T: Trait> balances::EnsureAccountLiquid<T::AccountId> for Module<T> {
    fn ensure_account_liquid(who: &T::AccountId) -> Result {
        if Self::bondage(who) <= <system::Module<T>>::block_number() {
            Ok(())
        } else {
            Err("cannot transfer illiquid funds")
        }
    }
}

impl<T: Trait> balances::OnFreeBalanceZero<T::AccountId> for Module<T> {
    fn on_free_balance_zero(who: &T::AccountId) {
        <Bondage<T>>::remove(who);
    }
}

impl<T: Trait> consensus::OnOfflineValidator for Module<T> {
    fn on_offline_validator(validator_index: usize) {
        let v = <session::Module<T>>::validators()[validator_index].clone();
        let slash_count = Self::slash_count(&v);
        <SlashCount<T>>::insert(v.clone(), slash_count + 1);
        let grace = Self::offline_slash_grace();

        let event = if slash_count >= grace {
            let instances = slash_count - grace;
            let slash = Self::current_offline_slash() << instances;
            let next_slash = slash << 1u32;
            let _ = Self::slash_validator(&v, slash);
            if instances >= Self::validator_preferences(&v).unstake_threshold
                || Self::slashable_balance(&v) < next_slash
            {
                if let Some(pos) = Self::intentions().into_iter().position(|x| &x == &v) {
                    Self::apply_unstake(&v, pos)
                        .expect("pos derived correctly from Self::intentions(); \
                            apply_unstake can only fail if pos wrong; \
                            Self::intentions() doesn't change; qed");
                }
                let _ = Self::apply_force_new_era(false);
            }
            RawEvent::OfflineSlash(v, slash)
        } else {
            RawEvent::OfflineWarning(v, slash_count)
        };
        Self::deposit_event(event);
    }
}
