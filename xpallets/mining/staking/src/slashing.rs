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

    fn reward_per_block(staking_reward: T::Balance, validator_count: usize) -> u128 {
        let session_length = T::SessionDuration::get();
        let per_reward = staking_reward.saturated_into::<u128>()
            * validator_count.saturated_into::<u128>()
            / session_length.saturated_into::<u128>();
        per_reward
    }

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

    fn expected_slash_of(offender: &T::AccountId, reward_per_block: u128) -> T::Balance {
        let offence_cnt = OffenceCountInSession::<T>::take(offender);
        let ideal_slash =
            reward_per_block * u128::from(offence_cnt) * u128::from(Self::offence_severity());
        let min_slash = Self::minimum_penalty().saturated_into::<u128>() * u128::from(offence_cnt);
        let expected_slash = sp_std::cmp::max(ideal_slash, min_slash);
        expected_slash.saturated_into()
    }

    pub(crate) fn slash_offenders_in_session(staking_reward: T::Balance) -> u64 {
        // Find the offenders that are in the current validator set.
        let validators = T::SessionInterface::validators();
        let valid_offenders = Self::offenders_in_session()
            .into_iter()
            .filter(|o| validators.contains(o))
            .collect::<Vec<_>>();

        let reward_per_block = Self::reward_per_block(staking_reward, validators.len());

        let active_potential_validators = Validators::<T>::iter()
            .map(|(v, _)| v)
            .filter(Self::is_active)
            .collect::<Vec<_>>();

        let mut active_count = active_potential_validators.len();

        let mut force_chilled = 0;

        let minimum_validator_count = Self::minimum_validator_count() as usize;

        for offender in valid_offenders.iter() {
            let expected_slash = Self::expected_slash_of(offender, reward_per_block);
            if let Err(actual_slashed) = Self::try_slash(offender, expected_slash) {
                debug!(
                    "[slash_offenders_in_session]expected_slash:{:?}, actual_slashed:{:?}",
                    expected_slash, actual_slashed
                );
                if active_count > minimum_validator_count {
                    Self::apply_force_chilled(offender);
                    active_count -= 1;
                    force_chilled += 1;
                }
            }
        }

        force_chilled
    }
}
