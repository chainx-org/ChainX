// Copyright 2018 Chainpool.
//! Staking manager: Periodically determines the best set of validators.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;

#[cfg(feature = "std")]
extern crate serde_derive;

#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
extern crate sr_std as rstd;

#[macro_use]
extern crate srml_support as runtime_support;
#[cfg(test)]
extern crate srml_balances as balances;
#[cfg(test)]
extern crate srml_consensus as consensus;
extern crate srml_session as session;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

extern crate xrml_xaccounts as xaccounts;
extern crate xrml_xassets_assets as xassets;
extern crate xrml_xsupport as xsupport;

#[cfg(test)]
extern crate substrate_primitives;

use codec::{Compact, HasCompact};
use rstd::prelude::*;
use runtime_primitives::{
    traits::{As, CheckedAdd, CheckedSub, Zero},
    Perbill,
};
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};
use system::ensure_signed;

use xassets::Address;
use xsupport::storage::btree_map::CodecBTreeMap;

pub mod vote_weight;

mod shifter;

mod mock;

mod tests;

pub use vote_weight::{Jackpot, VoteWeight};

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;

/// Intention mutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct IntentionProfs<Balance: Default, BlockNumber: Default> {
    pub jackpot: Balance,
    pub total_nomination: Balance,
    pub last_total_vote_weight: u64,
    pub last_total_vote_weight_update: BlockNumber,
}

/// Nomination record of one of the nominator's nominations.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct NominationRecord<Balance: Default, BlockNumber: Default> {
    pub nomination: Balance,
    pub last_vote_weight: u64,
    pub last_vote_weight_update: BlockNumber,
}

pub trait Trait: xassets::Trait + xaccounts::Trait + session::Trait + timestamp::Trait {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        /// Transactor could be an intention.
        fn nominate(origin, target: Address<T::AccountId, T::AccountIndex>, value: T::Balance) {
            let who = ensure_signed(origin)?;
            let target = <xassets::Module<T>>::lookup(target)?;

            ensure!(!value.is_zero(), "Cannot nominate zero.");
            ensure!(
                <xaccounts::Module<T>>::intention_immutable_props_of(&target).is_some(),
                "Cannot nominate a non-intention."
            );
            ensure!(
                value <= <xassets::Module<T>>::pcx_free_balance(&who),
                "Cannot nominate if greater than your avaliable free balance."
            );

            Self::apply_nominate(&who, &target, value)?;
        }

        fn unnominate(origin, target: Address<T::AccountId, T::AccountIndex>, value: T::Balance) {
            let who = ensure_signed(origin)?;
            let target = <xassets::Module<T>>::lookup(target)?;

            ensure!(!value.is_zero(), "Cannot unnominate zero.");
            ensure!(
                Self::nominees_of(&who).iter().find(|&n| n == &target).is_some(),
                "Cannot unnominate if target is not your nominee."
            );
            ensure!(
                value <= Self::revokable_of(&who, &target),
                "Cannot unnominate if greater than your revokable nomination."
            );

            Self::apply_unnominate(&who, &target, value)?;
        }

        fn claim(origin, target: Address<T::AccountId, T::AccountIndex>) {
            let who = ensure_signed(origin)?;
            let target = <xassets::Module<T>>::lookup(target)?;

            ensure!(
                Self::nominees_of(&who).iter().find(|&n| n == &target).is_some(),
                "Cannot claim if target is not your nominee."
            );

            Self::apply_claim(&who, &target)?;
        }

        fn unfreeze(origin) {
            let who = ensure_signed(origin)?;

            let mut frozens = Self::remaining_frozen_of(&who);
            let current_block = <system::Module<T>>::block_number();

            for block in frozens.clone().into_iter() {
                if current_block > block {
                    let value = <FrozenValueOf<T>>::take((who.clone(), block));
                    <xassets::Module<T>>::pcx_staking_unreserve(&who, value)?;
                }
            }

            frozens.retain(|&n| n >= current_block);
            <RemainingFrozenOf<T>>::insert(&who, frozens);
        }

        /// Update the url and desire to join in elections of intention.
        fn refresh(origin, url: Vec<u8>, desire_to_run: bool) {
            let who = ensure_signed(origin)?;

            <xaccounts::Module<T>>::is_valid_url(&url)?;

            ensure!(
                <xaccounts::Module<T>>::intention_immutable_props_of(&who).is_some(),
                "Transactor is not an intention."
            );

            <xaccounts::IntentionPropertiesOf<T>>::mutate(&who, |props| {
                props.url = url;
                props.is_active = desire_to_run;
            });
        }

        /// Register intention by the owner of given cert name.
        fn register(origin, cert_name: Vec<u8>, intention: T::AccountId, name: Vec<u8>, url: Vec<u8>, share_count: u32) {
            let who = ensure_signed(origin)?;

            <xaccounts::Module<T>>::is_valid_name(&name)?;
            <xaccounts::Module<T>>::is_valid_url(&url)?;

            ensure!(share_count > 0, "Cannot register zero share.");
            ensure!(
                <xaccounts::Module<T>>::cert_owner_of(&cert_name).is_some(),
                "Cert name does not exist."
            );
            ensure!(
                <xaccounts::Module<T>>::cert_owner_of(&cert_name) == Some(who),
                "Transactor mismatches the owner of given cert name."
            );
            ensure!(
                <xaccounts::Module<T>>::remaining_shares_of(&cert_name) > 0,
                "Cannot register there are no remaining shares."
            );
            ensure!(
                <xaccounts::Module<T>>::intention_immutable_props_of(&intention).is_none(),
                "Cannot register an intention repeatedly."
            );

            Self::apply_register(cert_name, intention, name, url, share_count)?;

        }

        /// Set the number of sessions in an era.
        fn set_sessions_per_era(new: <T::BlockNumber as HasCompact>::Type) {
            <NextSessionsPerEra<T>>::put(new.into());
        }

        /// The length of the bonding duration in eras.
        fn set_bonding_duration(new: <T::BlockNumber as HasCompact>::Type) {
            <BondingDuration<T>>::put(new.into());
        }

        /// The ideal number of validators.
        fn set_validator_count(new: Compact<u32>) {
            let new: u32 = new.into();
            <ValidatorCount<T>>::put(new);
        }

        /// Force there to be a new era. This also forces a new session immediately after.
        /// `apply_rewards` should be true for validators to get the session reward.
        fn force_new_era(apply_rewards: bool) -> Result {
            Self::apply_force_new_era(apply_rewards)
        }

        /// Set the offline slash grace period.
        fn set_offline_slash_grace(new: Compact<u32>) {
            let new: u32 = new.into();
            <OfflineSlashGrace<T>>::put(new);
        }

    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Staking {
        /// The ideal number of staking participants.
        pub ValidatorCount get(validator_count) config(): u32;
        /// Minimum number of staking participants before emergency conditions are imposed.
        pub MinimumValidatorCount get(minimum_validator_count) config(): u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;
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

        pub StakeWeight get(stake_weight): map T::AccountId => T::Balance;

        pub TotalStake get(total_stake): T::Balance;

        pub NomineesOf get(nominees_of): map T::AccountId => Vec<T::AccountId>;

        pub IntentionProfiles get(intention_profiles): map T::AccountId => IntentionProfs<T::Balance, T::BlockNumber>;

        pub NominationRecords get(nomination_records): map T::AccountId => CodecBTreeMap<T::AccountId, NominationRecord<T::Balance, T::BlockNumber>>;

        pub RemainingFrozenOf get(remaining_frozen_of): map T::AccountId => Vec<T::BlockNumber>;

        pub FrozenValueOf get(frozen_value_of): map (T::AccountId, T::BlockNumber) => T::Balance;
    }
}

impl<T: Trait> Module<T> {
    // Public immutables

    /// Due of allocated shares that cert comes with.
    pub fn unfreeze_block_of(cert_name: Vec<u8>) -> T::BlockNumber {
        let props = <xaccounts::Module<T>>::cert_immutable_props_of(cert_name);
        let issued_at = props.issued_at;
        let frozen_duration = props.frozen_duration;
        let period = <timestamp::Module<T>>::block_period();
        let seconds = (frozen_duration * 24 * 60 * 60) as u64;

        issued_at + T::BlockNumber::sa(seconds / period.as_())
    }

    /// If source is an intention, the revokable balance should take the frozen duration
    /// of his activator into account.
    pub fn revokable_of(source: &T::AccountId, target: &T::AccountId) -> T::Balance {
        match <xaccounts::Module<T>>::intention_immutable_props_of(source) {
            Some(props) => {
                let activator = props.activator;

                let block_of_due = Self::unfreeze_block_of(activator);
                let current_block = <system::Module<T>>::block_number();

                // Should exclude the startup if still during the frozen duration.
                match block_of_due >= current_block {
                    true => {
                        let startup = T::Balance::sa(
                            props.initial_shares as u64
                                * <xaccounts::Module<T>>::activation_per_share() as u64,
                        );
                        Self::nomination_record_of(source, target).nomination - startup
                    }
                    false => Self::nomination_record_of(source, target).nomination,
                }
            }
            None => Self::nomination_record_of(source, target).nomination,
        }
    }

    /// How many votes nominator have nomianted for the nominee.
    pub fn nomination_record_of(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
    ) -> NominationRecord<T::Balance, T::BlockNumber> {
        if let Some(record) = <NominationRecords<T>>::get(nominator).0.get(nominee) {
            return record.clone();
        }
        <NominationRecord<T::Balance, T::BlockNumber>>::default()
    }

    pub fn total_nomination_of(intention: &T::AccountId) -> T::Balance {
        <IntentionProfiles<T>>::get(intention).total_nomination
    }

    // Public mutables

    /// Increase TotalStake by Value.
    pub fn increase_total_stake_by(value: T::Balance) {
        if let Some(v) = <Module<T>>::total_stake().checked_add(&value) {
            <TotalStake<T>>::put(v);
        }
    }

    /// Decrease TotalStake by Value.
    pub fn decrease_total_stake_by(value: T::Balance) {
        if let Some(v) = <Module<T>>::total_stake().checked_sub(&value) {
            <TotalStake<T>>::put(v);
        }
    }

    // Private mutables

    fn mutate_nomination_record(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
        record: NominationRecord<T::Balance, T::BlockNumber>,
    ) {
        let mut nominations = <NominationRecords<T>>::get(nominator);
        nominations.0.insert(nominee.clone(), record);
        <NominationRecords<T>>::insert(nominator, nominations);
    }

    // Just force_new_era without origin check.
    fn apply_force_new_era(apply_rewards: bool) -> Result {
        <ForcingNewEra<T>>::put(());
        <session::Module<T>>::apply_force_new_session(apply_rewards)
    }

    fn apply_nominate(source: &T::AccountId, target: &T::AccountId, value: T::Balance) -> Result {
        <xassets::Module<T>>::pcx_staking_reserve(source, value)?;

        Self::apply_update_vote_weight(source, target, value, true);

        let mut nominees = Self::nominees_of(source);
        if nominees.iter().find(|&n| n == target).is_none() {
            nominees.push(target.clone());
        }
        <NomineesOf<T>>::insert(source, nominees);

        Ok(())
    }

    fn apply_unnominate(source: &T::AccountId, target: &T::AccountId, value: T::Balance) -> Result {
        let freeze_until = <system::Module<T>>::block_number() + Self::bonding_duration();
        let mut blocks = Self::remaining_frozen_of(source);
        if blocks.iter().find(|&n| *n == freeze_until).is_none() {
            blocks.push(freeze_until);
        }

        <RemainingFrozenOf<T>>::insert(source, blocks);
        <FrozenValueOf<T>>::insert((source.clone(), freeze_until), value);

        Self::apply_update_vote_weight(source, target, value, false);

        Ok(())
    }

    fn apply_claim(who: &T::AccountId, target: &T::AccountId) -> Result {
        let mut iprof = <IntentionProfiles<T>>::get(target);
        let mut record = Self::nomination_record_of(who, target);

        Self::generic_claim(&mut record, &mut iprof, who)?;

        <IntentionProfiles<T>>::insert(target, iprof);
        Self::mutate_nomination_record(who, target, record);

        Ok(())
    }

    /// Actually register an intention.
    fn apply_register(
        cert_name: Vec<u8>,
        intention: T::AccountId,
        name: Vec<u8>,
        url: Vec<u8>,
        share_count: u32,
    ) -> Result {
        <xaccounts::IntentionOf<T>>::insert(&name, intention.clone());
        <xaccounts::RemainingSharesOf<T>>::mutate(&cert_name, |shares| *shares -= share_count);
        <xaccounts::IntentionImmutablePropertiesOf<T>>::insert(
            &intention,
            xaccounts::IntentionImmutableProps {
                name: name,
                activator: cert_name.clone(),
                initial_shares: share_count,
            },
        );
        <xaccounts::IntentionPropertiesOf<T>>::insert(
            &intention,
            xaccounts::IntentionProps {
                url: url,
                is_active: false,
            },
        );

        let free_balance = <xassets::Module<T>>::pcx_free_balance(&intention);
        let activation = share_count * <xaccounts::Module<T>>::activation_per_share();
        let activation = T::Balance::sa(activation as u64);
        <xassets::Module<T>>::pcx_set_free_balance(&intention, free_balance + activation);
        <xassets::Module<T>>::increase_total_stake_by(activation);

        Self::apply_nominate(&intention, &intention, activation)?;

        <Intentions<T>>::mutate(|i| i.push(intention.clone()));

        Ok(())
    }

    /// Actually update the vote weight of source and target.
    fn apply_update_vote_weight(
        source: &T::AccountId,
        target: &T::AccountId,
        value: T::Balance,
        to_add: bool,
    ) {
        let mut iprof = <IntentionProfiles<T>>::get(target);
        let mut record = Self::nomination_record_of(source, target);

        Self::update_vote_weight_both_way(&mut iprof, &mut record, value.as_() as u128, to_add);

        match to_add {
            true => Self::increase_total_stake_by(value),
            false => Self::decrease_total_stake_by(value),
        }

        <IntentionProfiles<T>>::insert(target, iprof);
        Self::mutate_nomination_record(source, target, record);
    }
}
