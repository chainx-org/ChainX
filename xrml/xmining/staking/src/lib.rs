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
extern crate srml_balances as balances;
#[cfg(test)]
extern crate srml_consensus as consensus;
extern crate srml_session as session;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

extern crate xr_primitives;

extern crate xrml_xaccounts as xaccounts;
extern crate xrml_xassets_assets as xassets;
extern crate xrml_xsupport as xsupport;
extern crate xrml_xsystem as xsystem;

#[cfg(test)]
extern crate substrate_primitives;

use balances::OnDilution;
use codec::{Compact, HasCompact};
use rstd::prelude::*;
use runtime_primitives::{
    traits::{As, Zero},
    Perbill,
};
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};
use system::ensure_signed;

use xr_primitives::XString;

use xassets::{Address, Token};

pub mod vote_weight;

mod shifter;

mod mock;

mod tests;

pub use shifter::{OnReward, OnRewardCalculation, RewardHolder};
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
    pub revocations: Vec<(BlockNumber, Balance)>,
}

pub trait Trait:
    xassets::Trait + xaccounts::Trait + xsystem::Trait + session::Trait + timestamp::Trait
{
    /// Some tokens minted.
    type OnRewardMinted: OnDilution<<Self as balances::Trait>::Balance>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Need to calculate the reward for non-intentions.
    type OnRewardCalculation: OnRewardCalculation<Self::AccountId, Self::Balance>;

    /// Time to distribute reward
    type OnReward: OnReward<Self::AccountId, Self::Balance>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Transactor could be an intention.
        fn nominate(
            origin,
            target: Address<T::AccountId, T::AccountIndex>,
            value: T::Balance,
            memo: XString
        ) {
            let who = ensure_signed(origin)?;
            let target = <xassets::Module<T>>::lookup(target)?;

            xassets::is_valid_memo::<T>(&memo)?;
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

        fn unnominate(
            origin,
            target: Address<T::AccountId, T::AccountIndex>,
            value: T::Balance,
            memo: XString
        ) {
            let who = ensure_signed(origin)?;
            let target = <xassets::Module<T>>::lookup(target)?;

            xassets::is_valid_memo::<T>(&memo)?;
            ensure!(!value.is_zero(), "Cannot unnominate zero.");
            ensure!(
                <NominationRecords<T>>::get((who.clone(), target.clone())).is_some(),
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
                <NominationRecords<T>>::get((who.clone(), target.clone())).is_some(),
                "Cannot claim if target is not your nominee."
            );

            Self::apply_claim(&who, &target)?;
        }

        fn unfreeze(
            origin,
            target: Address<T::AccountId, T::AccountIndex>,
            revocation_index: u32
        ) {
            let who = ensure_signed(origin)?;
            let target = <xassets::Module<T>>::lookup(target)?;

            ensure!(
                <NominationRecords<T>>::get((who.clone(), target.clone())).is_some(),
                "Cannot unfreeze if target is not your nominee."
            );

            let record = Self::nomination_record_of(&who, &target);
            let mut revocations = record.revocations;

            ensure!(
                revocations.len() > 0,
                "Revocation list is empty"
            );

            ensure!(
                revocation_index < revocations.len() as u32,
                "Revocation index out of range."
            );

            let (block, value) = revocations[revocation_index as usize];
            let current_block = <system::Module<T>>::block_number();

            if current_block < block {
                return Err("The requested revocation is not due yet.");
            }

            <xassets::Module<T>>::pcx_staking_unreserve(&who, value)?;
            revocations.swap_remove(revocation_index as usize);
            if let Some(mut record) = <NominationRecords<T>>::get((who.clone(), target.clone())) {
                record.revocations = revocations;
                <NominationRecords<T>>::insert((who, target), record);
            }
        }

        /// Update the url and desire to join in elections of intention.
        fn refresh(origin, url: XString, desire_to_run: bool) {
            let who = ensure_signed(origin)?;

            xaccounts::is_valid_url::<T>(&url)?;

            ensure!(
                <xaccounts::Module<T>>::intention_immutable_props_of(&who).is_some(),
                "Transactor is not an intention."
            );

            match desire_to_run {
                true => {
                    if who == <xsystem::Module<T>>::banned_account() {
                        return Err("This account has been banned.");
                    }
                },
                false => {
                    let active = Self::intentions().into_iter()
                        .filter(|ref n| <xaccounts::Module<T>>::intention_props_of(n.clone()).is_active)
                        .collect::<Vec<_>>();
                    if active.len() <= Self::minimum_validator_count() as usize {
                        return Err("cannot pull out when there are too few active intentions");
                    }
                }
            }

            <xaccounts::IntentionPropertiesOf<T>>::mutate(&who, |props| {
                props.url = url;
                props.is_active = desire_to_run;
            });
        }

        /// Register intention by the owner of given cert name.
        fn register(
            origin,
            cert_name: XString,
            intention: T::AccountId,
            name: XString,
            url: XString,
            share_count: u32,
            memo: XString
        ) {
            let who = ensure_signed(origin)?;

            xaccounts::is_valid_name::<T>(&name)?;
            xaccounts::is_valid_url::<T>(&url)?;
            xassets::is_valid_memo::<T>(&memo)?;

            ensure!(share_count > 0, "Cannot register zero share.");
            ensure!(
                <xaccounts::Module<T>>::cert_owner_of(&cert_name).is_some(),
                "Cert name does not exist."
            );
            ensure!(
                <xaccounts::Module<T>>::cert_owner_of(&cert_name) == Some(who),
                "Transactor mismatches the owner of given cert name."
            );
            let remaining_shares = <xaccounts::Module<T>>::remaining_shares_of(&cert_name);
            ensure!(
                remaining_shares > 0,
                "Cannot register there are no remaining shares."
            );
            ensure!(
                share_count <= remaining_shares,
                "Cannot register if remaining shares is not adequate."
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

decl_storage! {
    trait Store for Module<T: Trait> as XStaking {
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

        pub IntentionProfiles get(intention_profiles): map T::AccountId => IntentionProfs<T::Balance, T::BlockNumber>;

        pub NominationRecords get(nomination_records): map (T::AccountId, T::AccountId) => Option<NominationRecord<T::Balance, T::BlockNumber>>;
    }
}

impl<T: Trait> Module<T> {
    // Public immutables
    fn blocks_per_day() -> u64 {
        let period = <timestamp::Module<T>>::block_period();
        let seconds = (24 * 60 * 60) as u64;
        seconds / period.as_()
    }

    /// Due of allocated shares that cert comes with.
    pub fn unfreeze_block_of(cert_name: XString) -> T::BlockNumber {
        let props = <xaccounts::Module<T>>::cert_immutable_props_of(cert_name);
        let issued_at = props.issued_at;
        let frozen_duration = props.frozen_duration;

        issued_at + T::BlockNumber::sa(frozen_duration as u64 * Self::blocks_per_day())
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
        let mut record = NominationRecord::default();
        record.last_vote_weight_update = <system::Module<T>>::block_number();
        <NominationRecords<T>>::get((nominator.clone(), nominee.clone())).unwrap_or(record)
    }

    pub fn total_nomination_of(intention: &T::AccountId) -> T::Balance {
        <IntentionProfiles<T>>::get(intention).total_nomination
    }

    // Private mutables

    fn mutate_nomination_record(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
        record: NominationRecord<T::Balance, T::BlockNumber>,
    ) {
        <NominationRecords<T>>::insert((nominator.clone(), nominee.clone()), record);
    }

    // Just force_new_era without origin check.
    fn apply_force_new_era(apply_rewards: bool) -> Result {
        <ForcingNewEra<T>>::put(());
        <session::Module<T>>::apply_force_new_session(apply_rewards)
    }

    fn apply_nominate(source: &T::AccountId, target: &T::AccountId, value: T::Balance) -> Result {
        <xassets::Module<T>>::pcx_staking_reserve(source, value)?;

        Self::apply_update_vote_weight(source, target, value, true);

        Ok(())
    }

    fn apply_unnominate(source: &T::AccountId, target: &T::AccountId, value: T::Balance) -> Result {
        let freeze_until = <system::Module<T>>::block_number() + Self::bonding_duration();

        let mut revocations = Self::nomination_record_of(source, target).revocations;

        if let Some(index) = revocations.iter().position(|&n| n.0 == freeze_until) {
            let (freeze_until, old_value) = revocations[index];
            revocations[index] = (freeze_until, old_value + value);
        } else {
            revocations.push((freeze_until, value));
        }

        if let Some(mut record) = <NominationRecords<T>>::get((source.clone(), target.clone())) {
            record.revocations = revocations;
            <NominationRecords<T>>::insert((source.clone(), target.clone()), record);
        }

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
        cert_name: XString,
        intention: T::AccountId,
        name: XString,
        url: XString,
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

        <Intentions<T>>::mutate(|i| i.push(intention.clone()));

        let iprof = IntentionProfs {
            jackpot: Zero::zero(),
            total_nomination: Zero::zero(),
            last_total_vote_weight: 0,
            last_total_vote_weight_update: <system::Module<T>>::block_number(),
        };
        <IntentionProfiles<T>>::insert(&intention, iprof);

        Self::apply_nominate(&intention, &intention, activation)?;

        Ok(())
    }

    /// Actually update the vote weight and nomination balance of source and target.
    fn apply_update_vote_weight(
        source: &T::AccountId,
        target: &T::AccountId,
        value: T::Balance,
        to_add: bool,
    ) {
        let mut iprof = <IntentionProfiles<T>>::get(target);
        let mut record = Self::nomination_record_of(source, target);

        Self::update_vote_weight_both_way(&mut iprof, &mut record, value.as_(), to_add);

        <IntentionProfiles<T>>::insert(target, iprof);
        Self::mutate_nomination_record(source, target, record);
    }
}
