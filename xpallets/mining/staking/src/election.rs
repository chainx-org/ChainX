// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;

impl<T: Trait> Module<T> {
    /// Returns a new validator set for the new era.
    pub(crate) fn new_era(start_session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        // Increment or set current era.
        let current_era = CurrentEra::mutate(|s| {
            *s = Some(s.map(|s| s + 1).unwrap_or(0));
            s.unwrap()
        });
        ErasStartSessionIndex::insert(&current_era, &start_session_index);

        // Set staking information for new era.
        let maybe_new_validators = Self::select_and_update_validators(current_era);
        debug!(
            "[new_era] era_index:{}, start_session_index:{}, maybe_new_validators:{:?}",
            current_era, start_session_index, maybe_new_validators
        );

        maybe_new_validators
    }

    /// Returns true if the (potential) validator is able to join in the election.
    ///
    /// Two requirements:
    /// 1. has the desire to win the election.
    /// 2. meets the threshold of a valid candidate.
    fn is_qualified_candidate(who: &T::AccountId) -> bool {
        Self::is_active(who) && Self::meet_candidate_threshold(who)
    }

    /// Returns true if the candidate meets the minimum candidate threshold.
    ///
    /// Otherwise the candidate will be **forced to be chilled**.
    fn meet_candidate_threshold(who: &T::AccountId) -> bool {
        let BondRequirement { self_bonded, total } = Self::validator_candidate_requirement();
        let threshold_satisfied =
            Self::validator_self_bonded(who) >= self_bonded && Self::total_votes_of(who) >= total;

        if !threshold_satisfied && Self::try_force_chilled(who).is_ok() {
            xp_logging::info!("[meet_candidate_threshold] Force {:?} to be inactive since it doesn't meet the minimum bond requirement", who);
        }

        threshold_satisfied
    }

    /// Filters out all the qualified validator candidates, sorted by the total nominations.
    fn filter_out_candidates() -> Vec<(BalanceOf<T>, T::AccountId)> {
        let mut candidates = Self::validator_set()
            .filter(Self::is_qualified_candidate)
            .map(|v| (Self::total_votes_of(&v), v))
            .collect::<Vec<_>>();
        candidates.sort_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));
        candidates
    }

    /// Selects the new validator set at the end of the era.
    ///
    /// Order potential validators by their total nominations and
    /// choose the top-most ValidatorCount::get() of them.
    ///
    /// This should only be called at the end of an era.
    fn select_and_update_validators(_current_era: EraIndex) -> Option<Vec<T::AccountId>> {
        // TODO: might move to offchain worker solution in the future.
        // Currently there is no performance issue practically.
        let candidates = Self::filter_out_candidates();
        debug!("[select_and_update_validators] candidates:{:?}", candidates);

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators.
        if candidates.len() < Self::reasonable_minimum_validator_count() as usize {
            return None;
        }

        let desired_validator_count = ValidatorCount::get() as usize;

        let validators = candidates
            .into_iter()
            .take(desired_validator_count)
            .map(|(_, v)| v)
            .collect::<Vec<_>>();

        // Always return Some(new_validators).
        Some(validators)
    }
}
