use super::*;

impl<T: Trait> Module<T> {
    /// Average reward for validator per block.
    fn reward_per_block(staking_reward: T::Balance, validator_count: usize) -> u128 {
        let session_length = T::SessionDuration::get();
        let per_reward = staking_reward.saturated_into::<u128>()
            * validator_count.saturated_into::<u128>()
            / session_length.saturated_into::<u128>();
        per_reward
    }

    /// TODO: flexiable slash according to slash fraction?
    fn expected_slash_of(reward_per_block: u128) -> T::Balance {
        let ideal_slash = reward_per_block * u128::from(Self::offence_severity());
        let min_slash = Self::minimum_penalty().saturated_into::<u128>();
        let expected_slash = sp_std::cmp::max(ideal_slash, min_slash);
        expected_slash.saturated_into()
    }

    pub(crate) fn slash_offenders_in_session(staking_reward: T::Balance) -> Vec<T::AccountId> {
        // Find the offenders that are in the current validator set.
        let validators = T::SessionInterface::validators();
        let valid_offenders = Self::offenders_in_session()
            .into_iter()
            .filter(|offender| validators.contains(offender))
            .collect::<Vec<_>>();

        let reward_per_block = Self::reward_per_block(staking_reward, validators.len());

        let treasury_account = T::TreasuryAccount::treasury_account();
        let slasher = Slasher::<T>::new(treasury_account);

        let minimum_validator_count = Self::minimum_validator_count() as usize;

        let active_validators = Self::active_validator_set().collect::<Vec<_>>();
        let mut active_count = active_validators.len();

        let force_chilled = valid_offenders
            .into_iter()
            .flat_map(|offender| {
                let expected_slash = Self::expected_slash_of(reward_per_block);
                match slasher.try_slash(&offender, expected_slash) {
                    Ok(_) => None, // Slash the offender successfuly.
                    Err(actual_slashed) => {
                        debug!(
                            "[slash_offenders_in_session]expected_slash:{:?}, actual_slashed:{:?}",
                            expected_slash, actual_slashed
                        );
                        if active_count > minimum_validator_count {
                            Self::apply_force_chilled(&offender);
                            active_count -= 1;
                            Some(offender) // The offender does not have enough balance for the slashing.
                        } else {
                            None
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        force_chilled
    }
}
