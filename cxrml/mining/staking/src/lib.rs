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

#[cfg(test)]
extern crate cxrml_associations;
extern crate cxrml_support as cxsupport;
extern crate cxrml_system;
extern crate cxrml_tokenbalances as tokenbalances;
extern crate parity_codec as codec;
extern crate sr_primitives as primitives;
extern crate srml_balances as balances;
extern crate srml_consensus as consensus;
extern crate srml_session as session;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

#[cfg(test)]
extern crate sr_io as runtime_io;
#[cfg(test)]
extern crate substrate_primitives;

use balances::{address::Address, OnDilution};
use codec::Codec;
use primitives::{
    traits::{As, OnFinalise, One, Zero},
    Perbill,
};
use rstd::prelude::*;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};
use session::OnSessionChange;
use system::ensure_signed;

use cxsupport::storage::btree_map::CodecBTreeMap;

pub mod vote_weight;

mod mock;
mod tests;

pub use vote_weight::{Jackpot, VoteWeight};

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
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Stats<AccountId: Default, Balance: Default> {
    pub nominator_count: u64,
    pub candidates: Vec<AccountId>,
    pub total_stake: Balance,
}

/// Profile of intention
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct IntentionProfs<Balance: Default, BlockNumber: Default> {
    pub is_active: bool,
    pub url: Vec<u8>,
    pub name: Vec<u8>,
    pub frozen: Balance,
    pub jackpot: Balance,
    pub activator_index: u32,
    pub total_nomination: Balance,
    pub last_total_vote_weight: u64,
    pub last_total_vote_weight_update: BlockNumber,
}

/// Profile of nominator, intention per se is a nominator.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct NominatorProfs<AccountId: Default, Balance: Default> {
    pub locked: Balance,
    pub nominees: Vec<AccountId>,
}

/// Nomination record of one of the nominator's nominations.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct NominationRecord<Balance: Default, BlockNumber: Default> {
    pub nomination: Balance,
    pub last_vote_weight: u64,
    pub last_vote_weight_update: BlockNumber,
}

/// Locked accounts composed of waiting to be unfreezed and unreserved at some block.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct LockedAccounts<AccountId: Default> {
    pub to_unfreeze: Vec<AccountId>,
    pub to_unreserve: Vec<AccountId>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct CertProfs<AccountId: Default, BlockNumber: Default> {
    pub name: Vec<u8>,
    pub index: u32,
    pub owner: AccountId,
    pub issued_on: BlockNumber,
    pub frozen_duration: u32,
    pub remaining_shares: u32,
}

pub trait Trait: balances::Trait + session::Trait + tokenbalances::Trait {
    /// Some tokens minted.
    type OnRewardMinted: OnDilution<<Self as balances::Trait>::Balance>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// trigger on new session, return token staking module (validator<token>, stake)
    type OnNewSessionForTokenStaking: OnNewSessionForTokenStaking<Self::AccountId, Self::Balance>;
    /// new session reward trigger
    type OnReward: OnReward<Self::AccountId, Self::Balance>;
}

pub trait OnNewSessionForTokenStaking<AccountId: Default + Codec, Balance> {
    fn token_staking_info() -> Vec<(Validator<AccountId>, Balance)>;
}

impl<AccountId: Default + Codec, Balance> OnNewSessionForTokenStaking<AccountId, Balance> for () {
    fn token_staking_info() -> Vec<(Validator<AccountId>, Balance)> {
        Vec::new()
    }
}

pub trait OnReward<AccountId: Default + Codec, Balance> {
    fn on_reward(v: &Validator<AccountId>, b: Balance);
}

impl<AccountId: Default + Codec, Balance> OnReward<AccountId, Balance> for () {
    fn on_reward(_: &Validator<AccountId>, _: Balance) {}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum Validator<AccountId: Default + Codec> {
    AccountId(AccountId),
    Token(tokenbalances::Symbol),
}

impl<AccountId: Default + Codec> Default for Validator<AccountId> {
    fn default() -> Self {
        Validator::AccountId(Default::default())
    }
}

decl_module! {
    #[cfg_attr(feature = "std", serde(bound(deserialize = "T::Balance: ::serde::de::DeserializeOwned")))]
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn stake(origin, value: T::Balance) -> Result;
        fn unstake(origin, value: T::Balance) -> Result;
        fn register(origin, cert_index: u32, intention: T::AccountId, name: Vec<u8>, url: Vec<u8>, share_count: u32) -> Result;
        fn activate(origin) -> Result;
        fn deactivate(origin) -> Result;
        fn claim(origin, target: Address<T::AccountId, T::AccountIndex>) -> Result;
        fn nominate(origin, target: Address<T::AccountId, T::AccountIndex>, value: T::Balance) -> Result;
        fn unnominate(origin, target: Address<T::AccountId, T::AccountIndex>, value: T::Balance) -> Result;
        fn register_preferences(origin, intentions_index: u32, prefs: ValidatorPrefs<T::Balance>) -> Result;

        fn issue(cert_name: Vec<u8>, frozen_duration: u32, cert_owner: T::AccountId) -> Result;

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
        pub MinimumValidatorCount get(minimum_validator_count) config(): u32;
        /// Maximum number of cert owners.
        pub MaximumCertOwnerCount get(maximum_cert_owner_count) config(): u32;
        /// Current cert owner index.
        pub CertOwnerIndex get(cert_owner_index): u32;
        /// Shares per cert.
        pub SharesPerCert get(shares_per_cert) config(): u32;
        /// Activation per share.
        pub ActivationPerShare get(activation_per_share) config(): u32;
        /// Maximum number of intentions.
        pub IntentionThreshold get(intention_threshold) config(): u32;
        /// The length of a staking era in sessions.
        pub SessionsPerEra get(sessions_per_era) config(): T::BlockNumber = T::BlockNumber::sa(1000);
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

        pub RegisterFee get(register_fee) config(): T::Balance;
        pub ClaimFee get(claim_fee) config(): T::Balance;
        pub StakeFee get(stake_fee) config(): T::Balance;
        pub UnstakeFee get(unstake_fee) config(): T::Balance;
        pub ActivateFee get(activate_fee) config(): T::Balance;
        pub DeactivateFee get(deactivate_fee) config(): T::Balance;
        pub NominateFee get(nominate_fee) config(): T::Balance;
        pub UnnominateFee get(unnominate_fee) config(): T::Balance;

        /// The next value of sessions per era.
        pub NextSessionsPerEra get(next_sessions_per_era): Option<T::BlockNumber>;
        /// The session index at which the era length last changed.
        pub LastEraLengthChange get(last_era_length_change): T::BlockNumber;

        /// We are forcing a new era.
        pub ForcingNewEra get(forcing_new_era): Option<()>;

        pub StakeWeight get(stake_weight): map T::AccountId => T::Balance;

        /// All (potential) validator -> reward for each session
//        pub SessionRewardOf get(session_reward_of): map T::AccountId => T::Balance;
        pub SessionRewardOf get(session_reward_of): map Validator<T::AccountId> => T::Balance;

        /// All intention -> profiles
        pub IntentionProfiles get(intention_profiles): map T::AccountId => IntentionProfs<T::Balance, T::BlockNumber>;
        /// All nominator -> profiles
        pub NominatorProfiles get(nominator_profiles): map T::AccountId => NominatorProfs<T::AccountId, T::Balance>;
        /// All nominator -> nomination records
        pub NominationRecords get(nomination_records): map T::AccountId => Nominations<T>;
        /// All certificate owners
        pub CertOwners get(cert_owners): Vec<T::AccountId>;
        /// All certificate profiles
        pub CertProfiles get(cert_profiles): map u32 => CertProfs<T::AccountId, T::BlockNumber>;

        /// Whole staking statistics
        pub StakingStats get(staking_stats): Stats<T::AccountId, T::Balance>;

        /// All block number -> accounts waiting to be unlocked at that block
        pub LockedAccountsOf get(locked_accounts_of): map T::BlockNumber => LockedAccounts<T::AccountId>;
        /// (nominator, unlock_block) => unlock_value
        pub LockedOf get(locked_of): map (T::AccountId, T::BlockNumber) => T::Balance;
    }

    add_extra_genesis {
        config(intention_profiles): Vec<(T::AccountId, Vec<u8>, Vec<u8>)>;
        config(cert_owner): T::AccountId;

        build(|storage: &mut primitives::StorageMap, config: &GenesisConfig<T>| {
            use codec::Encode;
            let mut stats: Stats<T::AccountId, T::Balance> = Stats::default();
            for (acnt, name, url) in config.intention_profiles.iter() {
                let mut iprof: IntentionProfs<T::Balance, T::BlockNumber> = IntentionProfs::default();
                iprof.name = name.clone();
                iprof.url = url.clone();
                iprof.is_active = true;
                iprof.total_nomination = T::Balance::sa(1);
                stats.total_stake += T::Balance::sa(1);
                storage.insert(GenesisConfig::<T>::hash(&<IntentionProfiles<T>>::key_for(acnt)).to_vec(), iprof.encode());
            }
            storage.insert(GenesisConfig::<T>::hash(&<StakingStats<T>>::key()).to_vec(), stats.encode());

            let mut cert: CertProfs<T::AccountId, T::BlockNumber> = CertProfs::default();
            cert.name = b"Alice".to_vec();
            cert.index = 0;
            cert.frozen_duration = 1;
            cert.remaining_shares = config.shares_per_cert;
            cert.owner = config.cert_owner.clone();
            storage.insert(GenesisConfig::<T>::hash(&<CertProfiles<T>>::key_for(0u32)).to_vec(), cert.encode());

        });
    }
}

impl<T: Trait> Module<T> {
    /// Total locked balance of a nominator, including the frozen part if he is also an intention.
    pub fn locked(who: &T::AccountId) -> T::Balance {
        <NominatorProfiles<T>>::get(who).locked + <IntentionProfiles<T>>::get(who).frozen
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

    pub fn total_nomination_of_intention(intention: &T::AccountId) -> T::Balance {
        <IntentionProfiles<T>>::get(intention).total_nomination
    }

    /// Latest vote weight of intention
    pub fn total_vote_weight_of_intention(intention: &T::AccountId) -> u64 {
        let iprof = <IntentionProfiles<T>>::get(intention);
        iprof.last_total_vote_weight
            + iprof.total_nomination.as_()
                * (<system::Module<T>>::block_number() - iprof.last_total_vote_weight_update).as_()
    }

    /// Latest vote weight of nominator to some nominee
    pub fn vote_weight_of(nominator: &T::AccountId, nominee: &T::AccountId) -> u64 {
        let record = Self::nomination_record_of(nominator, nominee);

        record.last_vote_weight
            + record.nomination.as_()
                * (<system::Module<T>>::block_number() - record.last_vote_weight_update).as_()
    }

    /// Nomination of a nominator to his some nominee
    pub fn nomination_of_nominator(nominator: &T::AccountId, nominee: &T::AccountId) -> T::Balance {
        if let Some(record) = <NominationRecords<T>>::get(nominator).0.get(nominee) {
            return record.nomination;
        }
        Zero::zero()
    }

    /// All funds a nominator has nominated to his nominees
    pub fn total_nomination_of_nominator(nominator: &T::AccountId) -> T::Balance {
        <NominatorProfiles<T>>::get(nominator)
            .nominees
            .into_iter()
            .map(|x| Self::nomination_of_nominator(nominator, &x))
            .fold(Zero::zero(), |acc: T::Balance, x| acc + x)
    }

    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }

    fn day_to_block(n: u32) -> T::BlockNumber {
        T::BlockNumber::sa((n * 24 * 60 * 60) as u64 / <timestamp::Module<T>>::block_period().as_())
    }

    // PUBLIC IMMUTABLES

    /// The length of a staking era in blocks.
    pub fn era_length() -> T::BlockNumber {
        Self::sessions_per_era() * <session::Module<T>>::length()
    }

    // PUBLIC DISPATCH

    /// Issue cert.
    pub fn issue(cert_name: Vec<u8>, frozen_duration: u32, cert_owner: T::AccountId) -> Result {
        tokenbalances::is_valid_symbol(&cert_name)?;

        let index = <CertOwnerIndex<T>>::get();
        if index >= Self::maximum_cert_owner_count() {
            return Err("cannot issue when there are too many cert owners.");
        }

        let mut cert = CertProfs::default();
        cert.name = cert_name.clone();
        cert.index = index;
        cert.owner = cert_owner.clone();
        cert.issued_on = <system::Module<T>>::block_number();
        cert.frozen_duration = frozen_duration;
        cert.remaining_shares = Self::shares_per_cert();

        <CertProfiles<T>>::insert(index, cert);
        <CertOwnerIndex<T>>::put(index + 1);

        if Self::cert_owners()
            .iter()
            .find(|&c| c == &cert_owner)
            .is_none()
        {
            let mut cert_owners = Self::cert_owners();
            cert_owners.push(cert_owner);
            <CertOwners<T>>::put(cert_owners);
        }

        Ok(())
    }

    /// Register intention by cert owner.
    /// Effects will be felt at the beginning of the next bra.
    fn register(
        origin: T::Origin,
        cert_index: u32,
        intention: T::AccountId,
        name: Vec<u8>,
        url: Vec<u8>,
        share_count: u32,
    ) -> Result {
        let who = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&who, Self::register_fee(), true, || Ok(()))?;

        ensure!(share_count > 0, "Cannot register zero share.");

        let cert = <CertProfiles<T>>::get(cert_index);
        ensure!(
            cert.owner == who,
            "Cannot register if owner of requested cert mismatches."
        );

        ensure!(
            cert.remaining_shares > 0,
            "Cannot register if there are no remaining shares."
        );
        ensure!(
            share_count <= cert.remaining_shares,
            "Cannot register if greater than your remaining shares."
        );

        tokenbalances::is_valid_symbol(&name)?;
        tokenbalances::is_valid_symbol(&url)?;

        ensure!(
            <IntentionProfiles<T>>::get(&intention).name.is_empty(),
            "Cannot register if already registered."
        );

        ensure!(
            Self::intentions().len() <= Self::intention_threshold() as usize,
            "Cannot register if there are too many intentions already."
        );

        let value = T::Balance::sa((share_count * Self::activation_per_share()) as u64);
        let free_balance = <balances::Module<T>>::free_balance(&intention);
        <balances::Module<T>>::set_free_balance(&intention, free_balance + value);
        <balances::Module<T>>::increase_total_stake_by(value);

        let mut iprof = <IntentionProfiles<T>>::get(&intention);
        let mut nprof = <NominatorProfiles<T>>::get(&intention);
        let mut intentions = <Intentions<T>>::get();
        let mut stats = <StakingStats<T>>::get();
        let frozen_until = cert.issued_on + Self::day_to_block(cert.frozen_duration);
        if <system::Module<T>>::block_number() < frozen_until {
            iprof.frozen = value;

            let mut accounts = <LockedAccountsOf<T>>::get(frozen_until);
            accounts.to_unfreeze.push(intention.clone());
            <LockedAccountsOf<T>>::insert(frozen_until, accounts);
        }

        iprof.activator_index = cert.index;

        nprof.nominees.push(intention.clone());

        intentions.push(intention.clone());

        stats.nominator_count += 1;

        let mut cert = <CertProfiles<T>>::get(cert_index);
        cert.remaining_shares -= share_count;
        <CertProfiles<T>>::insert(cert_index, cert);

        <IntentionProfiles<T>>::insert(&intention, iprof);
        <NominatorProfiles<T>>::insert(&intention, nprof);
        <Intentions<T>>::put(intentions);
        <StakingStats<T>>::put(stats);

        Self::apply_register_identity(&intention, name, url)?;
        Self::apply_stake(&intention, value)?;

        Ok(())
    }

    /// Show the desire to stake for the transactor.
    fn activate(origin: T::Origin) -> Result {
        let who = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&who, Self::activate_fee(), true, || Ok(()))?;

        ensure!(
            !<IntentionProfiles<T>>::get(&who).is_active,
            "Cannot activate if already active."
        );

        ensure!(
            Self::intentions().iter().find(|&t| t == &who).is_some(),
            "Cannot activate if transactor is not an intention."
        );

        let mut iprof = <IntentionProfiles<T>>::get(&who);
        iprof.is_active = true;
        <IntentionProfiles<T>>::insert(&who, iprof);

        Ok(())
    }

    /// Retract the desire to stake for the transactor.
    ///
    /// Effects will be felt at the beginning of the next era.
    fn deactivate(origin: T::Origin) -> Result {
        let who = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&who, Self::deactivate_fee(), true, || Ok(()))?;

        ensure!(
            <IntentionProfiles<T>>::get(&who).is_active,
            "Cannot deactivate if already inactive."
        );

        ensure!(
            Self::intentions().iter().find(|&t| t == &who).is_some(),
            "Cannot deactivate if transactor is not an intention."
        );

        // deactivate fails in degenerate case of having too few existing staked parties
        if Self::intentions().len() <= Self::minimum_validator_count() as usize {
            return Err("cannot deactivate when there are too few staked participants");
        }

        let mut iprof = <IntentionProfiles<T>>::get(&who);
        iprof.is_active = false;
        <IntentionProfiles<T>>::insert(who, iprof);

        Ok(())
    }

    /// Increase the stake
    fn stake(origin: T::Origin, value: T::Balance) -> Result {
        let who = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&who, Self::stake_fee(), true, || Ok(()))?;

        ensure!(value.as_() > 0, "Cannot stake zero.");

        ensure!(
            value <= <balances::Module<T>>::free_balance(&who),
            "Cannot stake if amount greater than your free balance."
        );

        ensure!(
            Self::intentions().iter().find(|&t| t == &who).is_some(),
            "Cannot stake if transactor is not an intention."
        );

        Self::apply_stake(&who, value)?;

        Ok(())
    }

    /// Decrease the stake
    fn unstake(origin: T::Origin, value: T::Balance) -> Result {
        let who = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&who, Self::unstake_fee(), true, || Ok(()))?;

        ensure!(value.as_() > 0, "Cannot unstake zero.");

        let current_nomination = Self::nomination_record_of(&who, &who).nomination;

        ensure!(
            value <= current_nomination,
            "Cannot unstake if amount greater than your current nomination."
        );

        ensure!(
            Self::intentions().iter().find(|&t| t == &who).is_some(),
            "Cannot unstake if transactor is not an intention."
        );

        let mut nprof = <NominatorProfiles<T>>::get(&who);
        if value == current_nomination {
            if let Some(index) = nprof.nominees.iter().position(|x| *x == who) {
                nprof.nominees.swap_remove(index);
            }
        }
        <NominatorProfiles<T>>::insert(&who, nprof);

        Self::apply_unstake(&who, value)?;

        Ok(())
    }

    fn nominate(
        origin: T::Origin,
        target: Address<T::AccountId, T::AccountIndex>,
        value: T::Balance,
    ) -> Result {
        let who = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&who, Self::nominate_fee(), true, || Ok(()))?;

        let target = <balances::Module<T>>::lookup(target)?;

        ensure!(value.as_() > 0, "Cannot stake zero.");

        ensure!(
            Self::intentions().iter().find(|&t| t == &target).is_some(),
            "cannot nominate if target is not an intention."
        );

        if Self::intentions().iter().find(|&t| t == &who).is_some() {
            ensure!(who != target, "cannot nominate per se as an intention.");
        }

        ensure!(
            value <= <balances::Module<T>>::free_balance(&who),
            "Cannot nominate if greater than your avaliable free balance."
        );

        // reserve nominated balance
        <balances::Module<T>>::reserve(&who, value)?;

        let mut iprof = <IntentionProfiles<T>>::get(&target);
        let mut nprof = <NominatorProfiles<T>>::get(&who);
        let mut record = Self::nomination_record_of(&who, &target);
        let mut stats = <StakingStats<T>>::get();

        Self::update_vote_weight_both_way(&mut iprof, &mut record, value.as_() as u128, true);

        stats.total_stake += value;

        if nprof.nominees.is_empty() {
            stats.nominator_count += 1;
        }

        // update relationships
        // if nominator nominates nominee for the first time
        if nprof.nominees.iter().find(|&n| n == &target).is_none() {
            nprof.nominees.push(target.clone());
        }

        <IntentionProfiles<T>>::insert(&target, iprof);
        <NominatorProfiles<T>>::insert(&who, nprof);
        Self::insert_nomination_record(&who, &target, record);
        <StakingStats<T>>::put(stats);

        Ok(())
    }

    /// Claim dividend from intention's jackpot
    fn claim(origin: T::Origin, target: Address<T::AccountId, T::AccountIndex>) -> Result {
        let source = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&source, Self::claim_fee(), true, || Ok(()))?;

        let target = <balances::Module<T>>::lookup(target)?;

        let nprof = <NominatorProfiles<T>>::get(&source);

        ensure!(
            nprof.nominees.iter().find(|&t| t == &target).is_some(),
            "Cannot claim if target is not your nominee."
        );

        let mut iprof = Self::intention_profiles(&target);
        let mut record = Self::nomination_record_of(&source, &target);

        Self::generic_claim(&mut record, &mut iprof, &source)?;

        <IntentionProfiles<T>>::insert(target.clone(), iprof);
        Self::insert_nomination_record(&source, &target, record);

        Ok(())
    }

    /// Will panic if called when source isn't currently nominating target.
    /// target_index is the index of nominee list, 4 => [3, 2], unnominate 3, target_index = 0
    fn unnominate(
        origin: T::Origin,
        target: Address<T::AccountId, T::AccountIndex>,
        value: T::Balance,
    ) -> Result {
        let source = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(
            &source,
            Self::unnominate_fee(),
            true,
            || Ok(()),
        )?;

        ensure!(value.as_() > 0, "Cannot unnominate zero.");

        let target = <balances::Module<T>>::lookup(target)?;

        let nprof = <NominatorProfiles<T>>::get(&source);

        ensure!(
            nprof.nominees.iter().find(|&t| t == &target).is_some(),
            "Cannot claim if target is not your nominee."
        );

        let mut record = Self::nomination_record_of(&source, &target);

        let current_nomination = record.nomination;
        ensure!(
            value <= current_nomination,
            "Cannot unnominate if the amount greater than your current nomination."
        );

        // Ok - all valid.

        let mut nprof = <NominatorProfiles<T>>::get(&source);
        let mut iprof = <IntentionProfiles<T>>::get(&target);
        let mut stats = <StakingStats<T>>::get();

        let current_block = <system::Module<T>>::block_number();

        // update relationships if withdraw all votes
        if value == current_nomination {
            if let Some(index) = nprof.nominees.iter().position(|x| *x == target.clone()) {
                nprof.nominees.swap_remove(index);
            }

            if nprof.nominees.is_empty() {
                stats.nominator_count -= 1;
            }
        }

        // update nominator profile
        nprof.locked += value;

        // update locked info
        let to_lock = value;
        let lock_until = current_block + Self::bonding_duration();

        Self::lazy_unreserve(&source, to_lock, lock_until);

        Self::update_vote_weight_both_way(&mut iprof, &mut record, value.as_() as u128, false);

        stats.total_stake -= value;

        <IntentionProfiles<T>>::insert(&target, iprof);
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

    /// Reward a given (potential) validator by a specific amount.
    /// Add the reward to their balance, and their jackpot, pro-rata.
    fn reward(who: &T::AccountId, reward: T::Balance) {
        let off_the_table = T::Balance::sa(reward.as_() * 1 / 10);
        let _ = <balances::Module<T>>::reward(who, off_the_table);
        let to_jackpot = reward - off_the_table;
        let mut iprof = <IntentionProfiles<T>>::get(who);
        iprof.jackpot += to_jackpot;
        <IntentionProfiles<T>>::insert(who, iprof);
    }

    /// Acutally carry out the register_identity operation.
    fn apply_register_identity(who: &T::AccountId, name: Vec<u8>, url: Vec<u8>) -> Result {
        let mut iprof = <IntentionProfiles<T>>::get(who);
        iprof.name = name;
        iprof.url = url;
        <IntentionProfiles<T>>::insert(who, iprof);

        Ok(())
    }

    /// Actually carry out the stake operation.
    /// Reserve the increased value.
    fn apply_stake(who: &T::AccountId, value: T::Balance) -> Result {
        let mut iprof = <IntentionProfiles<T>>::get(who);
        let mut stats = <StakingStats<T>>::get();
        let mut record = Self::nomination_record_of(who, who);

        <balances::Module<T>>::reserve(who, value)?;

        Self::update_vote_weight_both_way(&mut iprof, &mut record, value.as_() as u128, true);

        stats.total_stake += value;

        <IntentionProfiles<T>>::insert(who.clone(), iprof);
        <StakingStats<T>>::put(stats);
        Self::insert_nomination_record(who, who, record);

        Ok(())
    }

    /// Actually carry out the unstake operation.
    /// Unreserve the decreased value lazily.
    fn apply_unstake(who: &T::AccountId, value: T::Balance) -> Result {
        let mut iprof = <IntentionProfiles<T>>::get(who);
        let mut nprof = <NominatorProfiles<T>>::get(who);
        let mut stats = <StakingStats<T>>::get();
        let mut record = Self::nomination_record_of(who, who);

        let current_block = <system::Module<T>>::block_number();

        let to_lock = value;
        let lock_until = current_block + Self::bonding_duration();
        Self::lazy_unreserve(who, to_lock, lock_until);

        Self::update_vote_weight_both_way(&mut iprof, &mut record, value.as_() as u128, false);

        stats.total_stake -= value;

        nprof.locked += to_lock;

        <IntentionProfiles<T>>::insert(who.clone(), iprof);
        <NominatorProfiles<T>>::insert(who.clone(), nprof);
        <StakingStats<T>>::put(stats);
        Self::insert_nomination_record(&who, &who, record);

        Ok(())
    }

    /// Will unreserve the decreased stake automatically after the bonding duration.
    fn lazy_unreserve(who: &T::AccountId, to_lock: T::Balance, lock_until: T::BlockNumber) {
        // update the accounts to unreserve on the block
        let mut accounts = <LockedAccountsOf<T>>::get(lock_until);
        if accounts.to_unreserve.iter().find(|&a| a == who).is_none() {
            accounts.to_unreserve.push(who.clone());
            <LockedAccountsOf<T>>::insert(lock_until, accounts);
        }

        // accumulate all balance remaining reserved of a certain account on the block
        let locked = <LockedOf<T>>::get((who.clone(), lock_until));
        <LockedOf<T>>::insert((who.clone(), lock_until), locked + to_lock);
    }

    /// Get the reward for the session, assuming it ends with this block.
    fn this_session_reward() -> T::Balance {
        let total_stake = <StakingStats<T>>::get().total_stake.as_();
        let reward = match total_stake {
            0...100_000_000 => total_stake * 1 / 1000,
            100_000_001...200_000_000 => total_stake * 9 / 10000,
            200_000_001...300_000_000 => total_stake * 8 / 10000,
            300_000_001...400_000_000 => total_stake * 7 / 10000,
            400_000_001...500_000_000 => total_stake * 6 / 10000,
            500_000_001...600_000_000 => total_stake * 5 / 10000,
            600_000_001...700_000_000 => total_stake * 4 / 10000,
            700_000_001...800_000_000 => total_stake * 3 / 10000,
            800_000_001...900_000_000 => total_stake * 2 / 10000,
            _ => total_stake * 1 / 10000,
        };
        T::Balance::sa(reward)
    }

    /// Acutally unreserve the locked stake.
    fn unreserve(block_number: T::BlockNumber, to_unreserve: Vec<T::AccountId>) {
        for acnt in to_unreserve.into_iter() {
            let to_unlock = <LockedOf<T>>::take((acnt.clone(), block_number));
            <balances::Module<T>>::unreserve(&acnt, to_unlock);
            let mut nprof = <NominatorProfiles<T>>::get(&acnt);
            nprof.locked -= to_unlock;
            <NominatorProfiles<T>>::insert(&acnt, nprof);
        }
    }

    /// Unfreeze the initial start-up stake.
    fn unfreeze(to_unfreeze: Vec<T::AccountId>) {
        for acnt in to_unfreeze.into_iter() {
            let mut iprof = <IntentionProfiles<T>>::get(acnt.clone());
            iprof.frozen = Zero::zero();
            <IntentionProfiles<T>>::insert(acnt, iprof);
        }
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session(_actual_elapsed: T::Moment, should_reward: bool) {
        let block_number = <system::Module<T>>::block_number();
        let accounts = <LockedAccountsOf<T>>::take(block_number);
        let to_unreserve = accounts.to_unreserve;
        let to_unfreeze = accounts.to_unfreeze;

        Self::unreserve(block_number, to_unreserve);
        Self::unfreeze(to_unfreeze);

        if should_reward {
            // apply good session reward
            let reward = Self::this_session_reward();

            let mut total_minted: T::Balance = Zero::zero();

            let mut active_intentions: Vec<(Validator<T::AccountId>, T::Balance)> =
                Self::intentions()
                    .into_iter()
                    .filter(|i| Self::intention_profiles(i).is_active)
                    .map(|id| {
                        let s = Self::total_nomination_of_intention(&id);
                        // wrapper by Validator enum
                        (Validator::AccountId(id), s)
                    })
                    .collect::<Vec<_>>();

            // add other validator
            let token_list = T::OnNewSessionForTokenStaking::token_staking_info();
            active_intentions.extend(token_list);

            let total_active_stake = active_intentions
                .iter()
                .fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + *x);

            if !total_active_stake.is_zero() {
                for (v, s) in active_intentions.iter() {
                    let i_reward = *s * reward / total_active_stake;
                    // TODO session reward
                    <SessionRewardOf<T>>::insert(v, i_reward);
                    total_minted += i_reward;
                    match v {
                        Validator::AccountId(ref id) => Self::reward(id, i_reward),
                        _ => T::OnReward::on_reward(v, i_reward),
                    }
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
            .filter(|i| Self::intention_profiles(i).is_active)
            .map(|v| (Self::total_nomination_of_intention(&v), v))
            .collect::<Vec<_>>();

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators
        if intentions.len() < Self::minimum_validator_count() as usize {
            return;
        }

        intentions.sort_unstable_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));

        for (total_nomination, intention) in intentions.iter() {
            <StakeWeight<T>>::insert(intention, total_nomination.clone());
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
        <CurrentSessionReward<T>>::put(Self::this_session_reward());
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
