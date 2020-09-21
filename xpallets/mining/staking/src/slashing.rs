// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;

impl<T: Trait> Module<T> {
    /// Slash the offenders actually, returns the force chilled offenders.
    ///
    /// The slashed balances will be moved to the treasury.
    pub(crate) fn slash_offenders_in_session(
        offenders: Vec<T::AccountId>,
        validator_rewards: Vec<(T::AccountId, BalanceOf<T>)>,
    ) -> Vec<T::AccountId> {
        let validator_rewards = validator_rewards.into_iter().collect::<BTreeMap<_, _>>();

        let treasury_account = T::TreasuryAccount::treasury_account();
        let slasher = Slasher::<T>::new(treasury_account);

        let minimum_validator_count = Self::reasonable_minimum_validator_count() as usize;
        let minimum_penalty = Self::minimum_penalty();

        let mut active_count = Self::active_validator_set().count();

        offenders
            .into_iter()
            .filter(Self::is_active)
            .flat_map(|offender| {
                let penalty = validator_rewards
                    .get(&offender)
                    .copied()
                    .map(|base_slash| {
                        let penalty_value =
                            Self::offence_severity().saturated_into::<BalanceOf<T>>() * base_slash;
                        penalty_value.max(minimum_penalty)
                    })
                    .unwrap_or(minimum_penalty);
                match slasher.try_slash(&offender, penalty) {
                    Ok(_) => {
                        debug!(
                            "Slash the offender:{:?} for {:?} successfully",
                            offender, penalty
                        );
                        // Slash the offender successfully.
                        None
                    }
                    Err(actual_slashed) => {
                        debug!(
                            "Insufficient reward pot balance of {:?}, actual slashed:{:?}",
                            offender, actual_slashed
                        );
                        // Avoid the over-slashing, ensure the minimum active validators.
                        if active_count > minimum_validator_count {
                            Self::apply_force_chilled(&offender);
                            active_count -= 1;
                            // The offender does not have enough balance for the slashing.
                            Some(offender)
                        } else {
                            None
                        }
                    }
                }
            })
            .collect()
    }
}
