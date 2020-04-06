// Copyright 2018-2019 Chainpool.
//! Coordidate session and era rotation.

use super::*;

use primitives::traits::{One, Zero};
use xsession::OnSessionChange;
use xsupport::{debug, info, warn};
#[cfg(feature = "std")]
use xsupport::{validators, who};

impl<T: Trait> Module<T> {
    /// Gather all the active intentions sorted by total nomination.
    fn gather_candidates() -> Vec<(T::Balance, T::AccountId)> {
        let mut intentions = Self::intention_set()
            .into_iter()
            .filter(|v| Self::is_qualified_candidate(&v))
            .map(|v| (Self::total_nomination_of(&v), v))
            .collect::<Vec<_>>();
        intentions.sort_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));
        intentions
    }

    /// A qualified candidate for validator election should be active and reach the minimum candidate threshold.
    fn is_qualified_candidate(who: &T::AccountId) -> bool {
        Self::is_active(who) && Self::meet_candidate_threshold(who)
    }

    /// See if the minimum candidate threshold is satified, otherwise it will be forced to be inactive.
    fn meet_candidate_threshold(who: &T::AccountId) -> bool {
        let (self_bonded, total_bonded) = Self::minimum_candidate_threshold();
        let satisfy_the_threshold = Self::self_bonded_of(who) >= self_bonded
            && Self::total_nomination_of(who) >= total_bonded;

        if !satisfy_the_threshold && Self::try_force_inactive(who).is_ok() {
            info!("[meet_candidate_threshold] force {:?} to be inactive since it doesn't meet the minimum candidate threshold", who!(who));
        }

        satisfy_the_threshold
    }

    /// Report the total missed blocks info to the session module.
    fn report_total_missed_blocks_count() {
        if <xsession::SessionTotalMissedBlocksCount<T>>::exists() {
            warn!("[report_total_missed_blocks] xsession::SessionTotalMissedBlocksCount should not exist on new session.");
        }
        let total_missed = Self::offline_validators_per_session()
            .iter()
            .map(Self::missed_of_per_session)
            .sum::<u32>();
        <xsession::SessionTotalMissedBlocksCount<T>>::put(total_missed);
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session() {
        Self::report_total_missed_blocks_count();

        // No reward but only slash for these offline validators that are inactive atm.
        Self::slash_inactive_offline_validators();

        let mut validators = <xsession::Module<T>>::validators()
            .into_iter()
            .map(|(v, _)| v)
            .collect::<Vec<_>>();

        let current_validator_count = validators.len();

        // Try removing the evil validators first.
        let evil_validators = <EvilValidatorsPerSession<T>>::take();
        for evil_val in evil_validators.iter() {
            if validators.len() > Self::minimum_validator_count() as usize {
                validators.retain(|x| *x != *evil_val);
            }
        }

        Self::distribute_session_reward(&mut validators);

        // Reset slashed validator set
        <OfflineValidatorsPerSession<T>>::kill();

        let session_index = <xsession::Module<T>>::current_index();
        let is_new_era =
            ((session_index - Self::last_era_length_change()) % Self::sessions_per_era()).is_zero();

        if is_new_era {
            Self::new_era();
        } else if validators.len() < current_validator_count {
            Self::set_validators_on_non_era(validators);
        }
    }

    /// We only reduce the offline validators on non-era session.
    /// This happens when there are offline validators that are enforced to be inactive.
    pub fn set_validators_on_non_era(validators: Vec<T::AccountId>) {
        // Update to the latest total nomination
        let validators = validators
            .into_iter()
            .map(|v| (Self::intentions(&v).total_nomination.into(), v))
            .map(|(a, b)| (b, a))
            .collect::<Vec<_>>();
        <xsession::Module<T>>::set_validators(validators.as_slice());
        info!(
            "[set_validators_on_non_era] new validator set due to enforcing inactive: {:?}",
            validators!(validators)
        );
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
                <LastEraLengthChange<T>>::put(&<xsession::Module<T>>::current_index());
            }
        }

        // evaluate desired staking amounts and nominations and optimise to find the best
        // combination of validators, then use xsession::internal::set_validators().
        // for now, this just orders would-be stakers by their balances and chooses the top-most
        // <ValidatorCount<T>>::get() of them.
        // TODO: this is not sound. this should be moved to an off-chain solution mechanism.
        let candidates = Self::gather_candidates();

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators
        if candidates.len() < Self::minimum_validator_count() as usize {
            return;
        }

        for (total_nomination, intention) in candidates.iter() {
            <StakeWeight<T>>::insert(intention, *total_nomination);
        }

        let desired_validator_count = <ValidatorCount<T>>::get() as usize;

        let validators = candidates
            .clone()
            .into_iter()
            .take(desired_validator_count)
            .map(|(stake_weight, account_id)| (account_id, stake_weight.into()))
            .collect::<Vec<(_, _)>>();

        info!("[new_era] new validator set: {:?}", validators!(validators));
        <xsession::Module<T>>::set_validators(&validators);
        Self::deposit_event(RawEvent::Rotation(validators));
    }

    /// We only note these offline validators that are still active at the moment.
    pub fn on_offline_validator(v: &T::AccountId) {
        if !Self::is_active(v) {
            return;
        }

        debug!(
            "[note_offline_validator]: active offline validator noted: {:?}",
            who!(v)
        );

        <OfflineValidatorsPerSession<T>>::mutate(|offline| {
            if !offline.contains(v) {
                offline.push(v.clone())
            }
        });

        let missed = Self::missed_of_per_session(v);
        <MissedOfPerSession<T>>::insert(v, missed + 1);
    }
}

impl<T: Trait> OnSessionChange<T::Moment> for Module<T> {
    fn on_session_change() {
        Self::new_session();
    }
}

impl<T: Trait> consensus::OnOfflineReport<Vec<u32>> for Module<T> {
    fn handle_report(reported_indices: Vec<u32>) {
        for validator_index in reported_indices {
            let v = <xsession::Module<T>>::validators()[validator_index as usize].clone();
            Self::on_offline_validator(&v.0);
        }
    }
}
