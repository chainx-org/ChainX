// Copyright 2018-2019 Chainpool.
//! Staking manager: Periodically determines the best set of validators.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

mod mock;
mod reward;
mod shifter;
pub mod slash;
mod tests;
pub mod traits;
pub mod types;
pub mod vote_weight;

use parity_codec::Compact;

// Substrate
use primitives::traits::{As, Lookup, StaticLookup, Zero};
use rstd::prelude::*;
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, EnumerableStorageMap,
    StorageMap, StorageValue,
};
use system::ensure_signed;

// ChainX
use xaccounts::IntentionJackpotAccountIdFor;
use xassets::{AssetErr, Memo, Token};
use xr_primitives::{Name, XString, URL};
use xsession::SessionKeyUsability;
#[cfg(feature = "std")]
use xsupport::who;
use xsupport::{debug, error};

pub use self::traits::*;
pub use self::types::*;

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const SESSIONS_PER_ROUND: u64 = 210_000;

pub trait Trait:
    xsystem::Trait + xsession::Trait + xbridge_features::Trait + xsdot::Trait + xbridge_common::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Need to calculate the reward for non-intentions.
    type OnRewardCalculation: OnRewardCalculation<Self::AccountId, Self::Balance>;

    /// Time to distribute reward
    type OnReward: OnReward<Self::AccountId, Self::Balance>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Transactor could be an intention.
        pub fn nominate(
            origin,
            target: <T::Lookup as StaticLookup>::Source,
            value: T::Balance,
            memo: Memo
        ) {
            let who = ensure_signed(origin)?;
            let target = system::ChainContext::<T>::default().lookup(target)?;

            xassets::is_valid_memo::<T>(&memo)?;
            ensure!(!value.is_zero(), "Cannot nominate zero.");
            ensure!(
                Self::is_intention(&target),
                "Cannot nominate a non-intention."
            );
            ensure!(
                value <= <xassets::Module<T>>::pcx_free_balance(&who),
                "Cannot nominate if greater than your avaliable free balance."
            );

            if !Self::is_nominating_intention_itself(&who, &target) {
                Self::wont_reach_upper_bound(&target, value)?;
            }

            Self::apply_nominate(&who, &target, value)?;
        }

        /// Renominate from one to another intention.
        fn renominate(
            origin,
            from: <T::Lookup as StaticLookup>::Source,
            to: <T::Lookup as StaticLookup>::Source,
            value: T::Balance,
            memo: Memo
        ) {
            let who = ensure_signed(origin)?;
            let context = system::ChainContext::<T>::default();
            let from = context.lookup(from)?;
            let to = context.lookup(to)?;

            xassets::is_valid_memo::<T>(&memo)?;
            ensure!(!value.is_zero(), "Cannot renominate zero.");
            ensure!(
                <NominationRecords<T>>::get((who.clone(), from.clone())).is_some(),
                "Cannot renominate if the from party is not your nominee."
            );
            if Self::is_intention(&who) && who == from {
                return Err("Cannot renominate the intention self-bonded.");
            }
            ensure!(
                value <= Self::revokable_of(&who, &from),
                "Cannot renominate if greater than your current nomination."
            );

            if !Self::is_nominating_intention_itself(&who, &to) {
                Self::wont_reach_upper_bound(&to, value)?;
            }

            let bonding_duration = Self::bonding_duration();
            let current_block = <system::Module<T>>::block_number();
            if let Some(last_renomination) = Self::last_renomination_of(&who) {
                ensure!(current_block > last_renomination + bonding_duration, "Cannot renominate if your last renomination is not expired.");
            }

            Self::apply_renominate(&who, &from, &to, value, current_block)?;
        }

        /// Unbond the nomination.
        fn unnominate(
            origin,
            target: <T::Lookup as StaticLookup>::Source,
            value: T::Balance,
            memo: Memo
        ) {
            let who = ensure_signed(origin)?;
            let target = system::ChainContext::<T>::default().lookup(target)?;

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
            ensure!(
                Self::current_revocations_count(&who, &target) < Self::max_unbond_entries_per_intention() as usize,
                "Cannot unnomiate if the limit of max unbond entries is reached."
            );

            Self::apply_unnominate(&who, &target, value)?;
        }

        /// Claim the reward for your nomination.
        fn claim(origin, target: <T::Lookup as StaticLookup>::Source) {
            let who = ensure_signed(origin)?;
            let target = system::ChainContext::<T>::default().lookup(target)?;

            ensure!(
                <NominationRecords<T>>::get((who.clone(), target.clone())).is_some(),
                "Cannot claim if target is not your nominee."
            );

            Self::apply_claim(&who, &target)?;
        }

        /// Free the locked unnomination.
        fn unfreeze(
            origin,
            target: <T::Lookup as StaticLookup>::Source,
            revocation_index: u32
        ) {
            let who = ensure_signed(origin)?;
            let target = system::ChainContext::<T>::default().lookup(target)?;

            let nominate_pair = (who.clone(), target.clone());

            ensure!(
                <NominationRecords<T>>::get(&nominate_pair).is_some(),
                "Cannot unfreeze if target is not your nominee."
            );

            let record = Self::nomination_record_of(&who, &target);
            let mut revocations = record.revocations;

            ensure!(!revocations.is_empty(), "Revocation list is empty");
            ensure!(
                revocation_index < revocations.len() as u32,
                "Revocation index out of range."
            );

            let (block, value) = revocations[revocation_index as usize];
            let current_block = <system::Module<T>>::block_number();
            if current_block < block {
                return Err("The requested revocation is not due yet.");
            }

            Self::staking_unreserve(&who, value)?;
            revocations.swap_remove(revocation_index as usize);
            if let Some(mut record) = <NominationRecords<T>>::get(&nominate_pair) {
                record.revocations = revocations;
                <NominationRecords<T>>::insert(&nominate_pair, record);
            }
            Self::deposit_event(RawEvent::Unfreeze(who, target));
        }

        /// Update the url, desire to join in elections of intention and session key.
        fn refresh(
            origin,
            url: Option<URL>,
            desire_to_run: Option<bool>,
            next_key: Option<T::SessionKey>,
            about: Option<XString>
        ) {
            let who = ensure_signed(origin)?;

            ensure!(Self::is_intention(&who), "Cannot refresh if transactor is not an intention.");

            if let Some(url) = url.as_ref() {
                xaccounts::is_valid_url(url)?;
            }

            if let Some(about) = about.as_ref() {
                xaccounts::is_valid_about(about)?;
            }

            if let Some(desire_to_run) = desire_to_run.as_ref() {
                if !desire_to_run && !Self::is_able_to_apply_inactive() {
                    return Err("Cannot pull out when there are too few active intentions.");
                }
            }

            let next_key = if let Some(next_key) = next_key.as_ref() {
                match <xsession::Module<T>>::check_session_key_usability(next_key) {
                    SessionKeyUsability::Unused => Some(next_key.clone()),
                    SessionKeyUsability::UsedBy(cur_owner) => {
                        // If this session key is already used by the transactor, set it to None to skip the meaningless writing.
                        if cur_owner == who {
                            None
                        } else {
                            return Err("This authority key is already used by some other intention.");
                        }
                    }
                }
            } else {
                None
            };

            Self::apply_refresh(&who, url, desire_to_run, next_key, about);
        }

        /// Register to be an intention.
        fn register(origin, name: Name) {
            let who = ensure_signed(origin)?;

            xaccounts::is_valid_name(&name)?;

            ensure!(!Self::is_intention(&who), "Cannot register if transactor is an intention already.");
            ensure!(!Self::name_exists(name.clone()), "This name has already been taken.");
            ensure!(Self::intention_set().len() < Self::maximum_intention_count() as usize, "Cannot register if there are already too many intentions");

            Self::apply_register(&who, name)?;
        }

        /// Set the number of sessions in an era.
        fn set_sessions_per_era(#[compact] new: T::BlockNumber) {
            <NextSessionsPerEra<T>>::put(new);
        }

        /// The length of the bonding duration in eras.
        fn set_bonding_duration(#[compact] new: T::BlockNumber) {
            <BondingDuration<T>>::put(new);
        }

        /// The ideal number of validators.
        fn set_validator_count(new: Compact<u32>) {
            let new: u32 = new.into();
            <ValidatorCount<T>>::put(new);
        }

        /// The severity of missed blocks per session.
        fn set_missed_blocks_severity(new: Compact<u32>) {
            let new: u32 = new.into();
            <MissedBlockSeverity<T>>::put(new);
        }

        /// The maximum number of intentions.
        fn set_maximum_intention_count(new: Compact<u32>) {
            let new: u32 = new.into();
            <MaximumIntentionCount<T>>::put(new);
        }

        /// Set the offline slash grace period.
        fn set_minimum_penalty(new: T::Balance) {
            <MinimumPenalty<T>>::put(new);
        }

        /// Set the distribution ratio between cross-chain assets and native assets.
        pub fn set_distribution_ratio(new: (u32, u32)) {
            ensure!(new.0 > 0 && new.1 > 0, "DistributionRatio can not be zero.");
            <DistributionRatio<T>>::put(new);
        }

        /// Set the minimum validator candidate threshold.
        fn set_minimum_candidate_threshold(new: (T::Balance, T::Balance)) {
            <MinimumCandidateThreshold<T>>::put(new);
        }

        /// Set the factor of intention's total nomination upper bond.
        fn set_upper_bond_factor(new: u32) {
            <UpperBoundFactor<T>>::put(new);
        }

    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as xassets::Trait>::Balance,
        <T as consensus::Trait>::SessionKey,
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber
    {
        /// All validators have been rewarded by the given balance.
        Reward(Balance, Balance),
        /// Missed blocks by each offline validator per session.
        MissedBlocksOfOfflineValidatorPerSession(Vec<(AccountId, u32)>),
        EnforceValidatorsInactive(Vec<AccountId>),
        Rotation(Vec<(AccountId, u64)>),
        Unnominate(BlockNumber),
        Nominate(AccountId, AccountId, Balance),
        Claim(u64, u64, Balance),
        Refresh(AccountId, Option<URL>, Option<bool>, Option<SessionKey>, Option<XString>),
        Unfreeze(AccountId, AccountId),
        /// All rewards issued to all (psedu-)intentions.
        SessionReward(Balance, Balance, Balance, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XStaking {
        pub InitialReward get(initial_reward) config(): T::Balance;

        /// The ideal number of staking participants.
        pub ValidatorCount get(validator_count) config(): u32;
        /// Minimum number of staking participants before emergency conditions are imposed.
        pub MinimumValidatorCount get(minimum_validator_count) config(): u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;
        /// Minimum value (self_bonded, total_bonded) to be a candidate of validator election.
        pub MinimumCandidateThreshold get(minimum_candidate_threshold) : (T::Balance, T::Balance);
        /// The length of a staking era in sessions.
        pub SessionsPerEra get(sessions_per_era) config(): T::BlockNumber = T::BlockNumber::sa(1000);
        /// The length of the bonding duration in blocks.
        pub BondingDuration get(bonding_duration) config(): T::BlockNumber = T::BlockNumber::sa(1000);
        /// The length of the bonding duration in blocks for intention.
        pub IntentionBondingDuration get(intention_bonding_duration) config(): T::BlockNumber = T::BlockNumber::sa(10_000);

        /// Maximum number of intentions.
        pub MaximumIntentionCount get(maximum_intention_count) config(): u32;

        pub SessionsPerEpoch get(sessions_per_epoch) config(): T::BlockNumber = T::BlockNumber::sa(10_000);

        /// The current era index.
        pub CurrentEra get(current_era) config(): T::BlockNumber;

        /// Allocation ratio of native asset and cross-chain assets.
        pub DistributionRatio get(distribution_ratio): (u32, u32) = (1u32, 1u32);

        /// The next value of sessions per era.
        pub NextSessionsPerEra get(next_sessions_per_era): Option<T::BlockNumber>;
        /// The session index at which the era length last changed.
        pub LastEraLengthChange get(last_era_length_change): T::BlockNumber;

        /// We are forcing a new era.
        pub ForcingNewEra get(forcing_new_era): Option<()>;

        pub StakeWeight get(stake_weight): map T::AccountId => T::Balance;

        /// All the accounts with a desire to stake.
        pub Intentions get(intentions): linked_map T::AccountId => IntentionProfs<T::Balance, T::BlockNumber>;

        pub NominationRecords get(nomination_records): map (T::AccountId, T::AccountId) => Option<NominationRecord<T::Balance, T::BlockNumber>>;

        /// The upper bound nominations of the intention that could absorb is up to the self-bonded.
        pub UpperBoundFactor get(upper_bound_factor): u32 = 10u32;

        /// Reported validators that did evil, reset per session.
        pub EvilValidatorsPerSession get(evil_validators): Vec<T::AccountId>;

        /// The height of user's last nomination.
        pub LastRenominationOf get(last_renomination_of): map T::AccountId => Option<T::BlockNumber>;

        /// The maximum ongoing unbond entries simultaneously against per intention.
        pub MaxUnbondEntriesPerIntention get(max_unbond_entries_per_intention): u32 = 10u32;

        /// Minimum penalty for each slash.
        pub MinimumPenalty get(minimum_penalty) config(): T::Balance;
        /// The active validators that have ever been offline per session.
        pub OfflineValidatorsPerSession get(offline_validators_per_session): Vec<T::AccountId>;
        /// Total blocks that each active validator missed in the current session.
        pub MissedOfPerSession get(missed_of_per_session): map T::AccountId => u32;
        /// The higher the severity, the more slash for missed blocks.
        pub MissedBlockSeverity get(missed_blocks_severity) config(): u32;
    }
}

impl<T: Trait> Module<T> {
    // Public immutables
    pub fn revokable_of(source: &T::AccountId, target: &T::AccountId) -> T::Balance {
        Self::nomination_record_of(source, target).nomination
    }

    /// How many votes nominator have nomianted for the nominee.
    pub fn nomination_record_of(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
    ) -> NominationRecord<T::Balance, T::BlockNumber> {
        let mut record = NominationRecord::default();
        record.last_vote_weight_update = <system::Module<T>>::block_number();
        <NominationRecords<T>>::get(&(nominator.clone(), nominee.clone())).unwrap_or(record)
    }

    pub fn self_bonded_of(who: &T::AccountId) -> T::Balance {
        if let Some(record) = <NominationRecords<T>>::get(&(who.clone(), who.clone())) {
            record.nomination
        } else {
            Default::default()
        }
    }

    pub fn upper_bound_of(who: &T::AccountId) -> T::Balance {
        Self::self_bonded_of(who) * T::Balance::sa(u64::from(Self::upper_bound_factor()))
    }

    pub fn total_nomination_of(intention: &T::AccountId) -> T::Balance {
        <Intentions<T>>::get(intention).total_nomination
    }

    pub fn is_intention(who: &T::AccountId) -> bool {
        <xaccounts::Module<T>>::intention_name_of(who).is_some()
    }

    pub fn name_exists(name: Name) -> bool {
        <xaccounts::Module<T>>::intention_of(name).is_some()
    }

    pub fn is_active(who: &T::AccountId) -> bool {
        <xaccounts::Module<T>>::intention_props_of(who).is_active
    }

    pub fn intention_set() -> Vec<T::AccountId> {
        <Intentions<T>>::enumerate()
            .map(|(account, _)| account)
            .collect()
    }

    pub fn is_able_to_apply_inactive() -> bool {
        let active = Self::intention_set()
            .into_iter()
            .filter(|n| Self::is_active(n))
            .collect::<Vec<_>>();
        active.len() > Self::minimum_validator_count() as usize
    }

    // Private mutables
    fn apply_inactive(who: &T::AccountId) {
        <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
            props.is_active = false;
            props.last_inactive_since = <system::Module<T>>::block_number();
        });
    }

    fn force_inactive_unsafe(who: &T::AccountId) {
        Self::apply_inactive(who);
    }

    fn try_force_inactive(who: &T::AccountId) -> Result {
        if !Self::is_able_to_apply_inactive() {
            return Err("Cannot force inactive when there are too few active intentions.");
        }
        Self::apply_inactive(who);
        Ok(())
    }

    fn mutate_nomination_record(
        nominator: &T::AccountId,
        nominee: &T::AccountId,
        record: NominationRecord<T::Balance, T::BlockNumber>,
    ) {
        <NominationRecords<T>>::insert(&(nominator.clone(), nominee.clone()), record);
    }

    fn staking_reserve(who: &T::AccountId, value: T::Balance) -> Result {
        <xassets::Module<T>>::pcx_move_balance(
            who,
            xassets::AssetType::Free,
            who,
            xassets::AssetType::ReservedStaking,
            value,
        )
        .map_err(AssetErr::info)
    }

    fn unnominate_reserve(who: &T::AccountId, value: T::Balance) -> Result {
        <xassets::Module<T>>::pcx_move_balance(
            who,
            xassets::AssetType::ReservedStaking,
            who,
            xassets::AssetType::ReservedStakingRevocation,
            value,
        )
        .map_err(AssetErr::info)
    }

    fn staking_unreserve(who: &T::AccountId, value: T::Balance) -> Result {
        <xassets::Module<T>>::pcx_move_balance(
            who,
            xassets::AssetType::ReservedStakingRevocation,
            who,
            xassets::AssetType::Free,
            value,
        )
        .map_err(AssetErr::info)
    }

    fn apply_nominate(source: &T::AccountId, target: &T::AccountId, value: T::Balance) -> Result {
        Self::staking_reserve(source, value)?;
        Self::apply_update_vote_weight(source, target, value, true);
        Self::deposit_event(RawEvent::Nominate(source.clone(), target.clone(), value));
        Ok(())
    }

    fn apply_renominate(
        who: &T::AccountId,
        from: &T::AccountId,
        to: &T::AccountId,
        value: T::Balance,
        current_block: T::BlockNumber,
    ) -> Result {
        Self::apply_update_vote_weight(who, from, value, false);
        Self::apply_update_vote_weight(who, to, value, true);
        <LastRenominationOf<T>>::insert(who, current_block);
        Ok(())
    }

    fn apply_unnominate(source: &T::AccountId, target: &T::AccountId, value: T::Balance) -> Result {
        let bonding_duration = if Self::is_intention(source) && *source == *target {
            Self::intention_bonding_duration()
        } else {
            Self::bonding_duration()
        };
        let freeze_until = <system::Module<T>>::block_number() + bonding_duration;

        let mut revocations = Self::nomination_record_of(source, target).revocations;

        if let Some(index) = revocations.iter().position(|&n| n.0 == freeze_until) {
            let (freeze_until, old_value) = revocations[index];
            revocations[index] = (freeze_until, old_value + value);
        } else {
            revocations.push((freeze_until, value));
        }

        Self::unnominate_reserve(source, value)?;

        let nr_key = (source.clone(), target.clone());
        if let Some(mut record) = <NominationRecords<T>>::get(&nr_key) {
            record.revocations = revocations;
            <NominationRecords<T>>::insert(&nr_key, record);
        }

        Self::apply_update_vote_weight(source, target, value, false);

        Self::deposit_event(RawEvent::Unnominate(freeze_until));

        Ok(())
    }

    fn apply_claim(who: &T::AccountId, target: &T::AccountId) -> Result {
        debug!(target: "claim", "[vote claim] who: {:?}, target: {:?}", who, who!(target));
        let mut iprof = <Intentions<T>>::get(target);
        let mut record = Self::nomination_record_of(who, target);

        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(target);
        let (source_vote_weight, target_vote_weight, dividend) = Self::generic_claim(
            &mut record,
            who,
            &mut iprof,
            &jackpot_addr,
            ClaimType::Intention,
        )?;
        Self::deposit_event(RawEvent::Claim(
            source_vote_weight,
            target_vote_weight,
            dividend,
        ));

        <Intentions<T>>::insert(target, iprof);
        Self::mutate_nomination_record(who, target, record);

        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn bootstrap_refresh(
        who: &T::AccountId,
        url: Option<URL>,
        desire_to_run: Option<bool>,
        next_key: Option<T::SessionKey>,
        about: Option<XString>,
    ) {
        Self::apply_refresh(who, url, desire_to_run, next_key, about)
    }

    fn apply_refresh(
        who: &T::AccountId,
        url: Option<URL>,
        desire_to_run: Option<bool>,
        next_key: Option<T::SessionKey>,
        about: Option<XString>,
    ) {
        if let Some(url) = url.clone() {
            <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
                props.url = url;
            });
        }
        if let Some(desire_to_run) = desire_to_run {
            <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
                props.is_active = desire_to_run;
                if !desire_to_run {
                    props.last_inactive_since = <system::Module<T>>::block_number();
                }
            });
        }
        if let Some(about) = about.clone() {
            <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
                props.about = about;
            });
        }
        if let Some(next_key) = next_key.clone() {
            <xsession::Module<T>>::set_key(who, &next_key);

            <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
                props.session_key = Some(next_key);
            });
        }

        Self::deposit_event(RawEvent::Refresh(
            who.clone(),
            url,
            desire_to_run,
            next_key,
            about,
        ));
    }

    fn wont_reach_upper_bound(nominee: &T::AccountId, value: T::Balance) -> Result {
        let total_nomination = Self::total_nomination_of(nominee);
        let upper_bound = Self::upper_bound_of(nominee);
        if total_nomination + value <= upper_bound {
            Ok(())
        } else {
            error!("Fail to (re)nominate, upper bound of nominee({:?}) is {:?}, current total_nomination: {:?}, want to nominate: {:?}", nominee, upper_bound, total_nomination, value);
            Err("Cannot (re)nominate if the target is reaching the upper bound of total nomination.")
        }
    }

    fn is_nominating_intention_itself(nominator: &T::AccountId, nominee: &T::AccountId) -> bool {
        Self::is_intention(nominator) && *nominator == *nominee
    }

    fn current_revocations_count(who: &T::AccountId, target: &T::AccountId) -> usize {
        Self::nomination_record_of(who, target).revocations.len()
    }

    #[cfg(feature = "std")]
    pub fn bootstrap_register(intention: &T::AccountId, name: Name) -> Result {
        Self::apply_register(intention, name)
    }

    /// Actually register an intention.
    fn apply_register(intention: &T::AccountId, name: Name) -> Result {
        <xaccounts::IntentionOf<T>>::insert(&name, intention.clone());
        <xaccounts::IntentionNameOf<T>>::insert(intention, name);
        let mut intention_props = xaccounts::IntentionProps::default();
        let block_number = <system::Module<T>>::block_number();
        intention_props.registered_at = block_number;
        intention_props.last_inactive_since = block_number;
        <xaccounts::IntentionPropertiesOf<T>>::insert(intention, intention_props);

        <Intentions<T>>::insert(
            intention,
            IntentionProfs {
                total_nomination: Zero::zero(),
                last_total_vote_weight: 0,
                last_total_vote_weight_update: <system::Module<T>>::block_number(),
            },
        );

        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn bootstrap_update_vote_weight(
        source: &T::AccountId,
        target: &T::AccountId,
        value: T::Balance,
        to_add: bool,
    ) {
        Self::apply_update_vote_weight(source, target, value, to_add)
    }

    /// Actually update the vote weight and nomination balance of source and target.
    fn apply_update_vote_weight(
        source: &T::AccountId,
        target: &T::AccountId,
        value: T::Balance,
        to_add: bool,
    ) {
        let mut iprof = <Intentions<T>>::get(target);
        let mut record = Self::nomination_record_of(source, target);

        Self::update_vote_weight_both_way(&mut iprof, &mut record, value.as_(), to_add);

        <Intentions<T>>::insert(target, iprof);
        Self::mutate_nomination_record(source, target, record);
    }
}

impl<T: Trait> Module<T> {
    pub fn validators() -> Vec<(T::AccountId, u64)> {
        xsession::Module::<T>::validators()
    }

    pub fn cross_chain_assets_are_growing_too_fast() -> rstd::result::Result<(u128, u128), ()> {
        let total_staked = Self::intention_set()
            .into_iter()
            .filter(|i| Self::is_active(i))
            .map(|id| Self::total_nomination_of(&id).as_())
            .sum::<u64>();

        let total_cross_chain_assets = T::OnRewardCalculation::psedu_intentions_info()
            .iter()
            .map(|(_, x)| x.as_())
            .sum::<u64>();

        Self::are_growing_too_fast(total_cross_chain_assets, total_staked)
    }

    pub fn jackpot_accountid_for_unsafe(who: &T::AccountId) -> T::AccountId {
        T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who)
    }

    pub fn multi_jackpot_accountid_for_unsafe(whos: &[T::AccountId]) -> Vec<T::AccountId> {
        whos.iter()
            .map(|who| T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who))
            .collect()
    }
}
