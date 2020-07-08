use super::*;

impl<T: Trait> Module<T> {
    pub(crate) fn new_era(start_session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        // Increment or set current era.
        let current_era = CurrentEra::mutate(|s| {
            *s = Some(s.map(|s| s + 1).unwrap_or(0));
            s.unwrap()
        });
        ErasStartSessionIndex::insert(&current_era, &start_session_index);

        // Set staking information for new era.
        let maybe_new_validators = Self::select_and_update_validators(current_era);

        maybe_new_validators
    }

    /// Returns true if the (potential) validator:
    /// 1. has the desire to win the election
    /// 2. meets the threshold of a valid candidate.
    fn is_qualified_candidate(who: &T::AccountId) -> bool {
        Self::is_active(who) && Self::meet_candidate_threshold(who)
    }

    /// Returns true if the candidate meets the minimum candidate threshold.
    ///
    /// **Otherwise the candidate will be forced to be chilled**.
    fn meet_candidate_threshold(who: &T::AccountId) -> bool {
        let BondRequirement { self_bonded, total } = Self::validator_bond_requirement();
        let threshold_satisfied =
            Self::validator_self_bonded(who) >= self_bonded && Self::total_votes_of(who) >= total;

        if !threshold_satisfied && Self::try_force_chilled(who).is_ok() {
            xpallet_support::info!("[meet_candidate_threshold] force {:?} to be inactive since it doesn't meet the minimum bond requirement", who);
        }

        threshold_satisfied
    }

    /// Filters out all the qualified validator candidates, sorted by the total nominations.
    fn filter_out_candidates() -> Vec<(T::Balance, T::AccountId)> {
        let mut candidates = Self::potential_validator_set()
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
    fn select_and_update_validators(current_era: EraIndex) -> Option<Vec<T::AccountId>> {
        // TODO: move to offchain worker solution.
        // Currently there is no performance issue practically.
        let candidates = Self::filter_out_candidates();

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators.
        if candidates.len() < Self::minimum_validator_count() as usize {
            return None;
        }

        let desired_validator_count = ValidatorCount::get() as usize;

        let validators = candidates
            .into_iter()
            .take(desired_validator_count)
            .map(|(_, v)| v)
            .collect::<Vec<_>>();

        Some(validators)
    }
}
