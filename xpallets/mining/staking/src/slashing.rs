// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;

impl<T: Trait> Module<T> {
    /// Average reward for validator per block.
    fn reward_per_block(staking_reward: BalanceOf<T>, validator_count: usize) -> u128 {
        let session_length = T::SessionDuration::get();
        staking_reward.saturated_into::<u128>() * validator_count.saturated_into::<u128>()
            / session_length.saturated_into::<u128>()
    }

    /// TODO: flexiable slash according to slash fraction?
    fn expected_slash_of(reward_per_block: u128) -> BalanceOf<T> {
        let ideal_slash = reward_per_block * u128::from(Self::offence_severity());
        let min_slash = Self::minimum_penalty().saturated_into::<u128>();
        let expected_slash = sp_std::cmp::max(ideal_slash, min_slash);
        expected_slash.saturated_into()
    }

    /// Slash the offenders actually, returns the force chilled offenders.
    ///
    /// The slashed balances will be moved to the treasury.
    pub(crate) fn slash_offenders_in_session(staking_reward: BalanceOf<T>) -> Vec<T::AccountId> {
        let validators = T::SessionInterface::validators();
        let reward_per_block = Self::reward_per_block(staking_reward, validators.len());

        let treasury_account = T::TreasuryAccount::treasury_account();
        let slasher = Slasher::<T>::new(treasury_account);

        let minimum_validator_count = Self::reasonable_minimum_validator_count() as usize;

        let mut active_count = Self::active_validator_set().count();

        Self::offenders_in_session()
            .into_iter()
            .filter(|offender| validators.contains(offender)) // FIXME: is this neccessary?
            .flat_map(|offender| {
                let expected_slash = Self::expected_slash_of(reward_per_block);
                match slasher.try_slash(&offender, expected_slash) {
                    Ok(_) => None, // Slash the offender successfully.
                    Err(actual_slashed) => {
                        debug!(
                            "[slash_offenders_in_session]expected_slash:{:?}, actual_slashed:{:?}",
                            expected_slash, actual_slashed
                        );
                        // Avoid the over-slashing, ensure the minimum active validators.
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
            .collect()
    }
}
