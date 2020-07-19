use super::*;

impl<T: Trait> Module<T> {
    /// Actually slash the account being punished, all slashed balance will go to the treasury.
    fn apply_slash(reward_pot: &T::AccountId, value: T::Balance) {
        // FIXME: cache the treasury_account?
        let treasury_account = T::TreasuryAccount::treasury_account();
        let _ = <xpallet_assets::Module<T>>::pcx_move_free_balance(
            reward_pot,
            &treasury_account,
            value,
        );
    }

    /// Average reward for validator per block.
    fn reward_per_block(staking_reward: T::Balance, validator_count: usize) -> u128 {
        let session_length = T::SessionDuration::get();
        let per_reward = staking_reward.saturated_into::<u128>()
            * validator_count.saturated_into::<u128>()
            / session_length.saturated_into::<u128>();
        per_reward
    }

    /// Returns Ok(_) if the reward pot of offender has enough balance to cover the slashing,
    /// otherwise slash the reward pot as much as possible and returns the value actually slashed.
    fn try_slash(offender: &T::AccountId, expected_slash: T::Balance) -> Result<(), T::Balance> {
        let reward_pot = Self::reward_pot_for(offender);
        let reward_pot_balance = <xpallet_assets::Module<T>>::pcx_free_balance(&reward_pot);

        if expected_slash <= reward_pot_balance {
            Self::apply_slash(&reward_pot, expected_slash);
            Ok(())
        } else {
            Self::apply_slash(&reward_pot, reward_pot_balance);
            Err(reward_pot_balance)
        }
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
            .filter(|o| validators.contains(o))
            .collect::<Vec<_>>();

        let reward_per_block = Self::reward_per_block(staking_reward, validators.len());

        let minimum_validator_count = Self::minimum_validator_count() as usize;

        let active_validators = Self::active_validator_set().collect::<Vec<_>>();
        let mut active_count = active_validators.len();

        let force_chilled = valid_offenders
            .into_iter()
            .flat_map(|offender| {
                let expected_slash = Self::expected_slash_of(reward_per_block);
                match Self::try_slash(&offender, expected_slash) {
                    Ok(_) => None,
                    Err(actual_slashed) => {
                        debug!(
                            "[slash_offenders_in_session]expected_slash:{:?}, actual_slashed:{:?}",
                            expected_slash, actual_slashed
                        );
                        if active_count > minimum_validator_count {
                            Self::apply_force_chilled(&offender);
                            active_count -= 1;
                            Some(offender)
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
