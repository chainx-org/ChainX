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

    /// Select a new validator set from the potential validator set.
    fn try_do_election() -> Option<Vec<T::AccountId>> {
        todo!("actually do election")
    }

    /// Select the new validator set at the end of the era.
    ///
    /// This should only be called at the end of an era.
    fn select_and_update_validators(current_era: EraIndex) -> Option<Vec<T::AccountId>> {
        Self::try_do_election()
    }
}
