// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_std::ops::Mul;

use super::*;

impl<T: Trait> Module<T> {
    /// Returns the force chilled offenders if any after applying the slashings.
    ///
    /// The slashed balances will be moved to the treasury.
    pub(crate) fn slash_offenders_in_session(
        offenders: BTreeMap<T::AccountId, Perbill>,
        validator_rewards: Vec<(T::AccountId, BalanceOf<T>)>,
    ) -> Vec<T::AccountId> {
        let validator_rewards = validator_rewards.into_iter().collect::<BTreeMap<_, _>>();

        let treasury_account = T::TreasuryAccount::treasury_account();
        let slasher = Slasher::<T>::new(treasury_account);

        let minimum_penalty = Self::minimum_penalty();
        let minimum_validator_count = Self::reasonable_minimum_validator_count() as usize;

        let mut active_count = Self::active_validator_set().count();

        offenders
            .into_iter()
            .filter(|(who, _)| Self::is_active(who))
            .flat_map(|(offender, slash_fraction)| {
                let pot = Self::reward_pot_for(&offender);
                let base_slash = slash_fraction.mul(Self::free_balance(&pot));
                let penalty = validator_rewards
                    .get(&offender)
                    .copied()
                    .map(|reward| reward + base_slash)
                    .unwrap_or(base_slash)
                    .max(minimum_penalty);
                match slasher.try_slash(&offender, penalty) {
                    Ok(_) => {
                        debug!(
                            "Slash the offender:{:?} for penalty {:?} by the given slash_fraction:{:?} successfully",
                            offender, penalty, slash_fraction
                        );
                        None
                    }
                    Err(actual_slashed) => {
                        debug!(
                            "Insufficient reward pot balance of {:?}, actual slashed:{:?}",
                            offender, actual_slashed
                        );
                        // The offender does not have enough balance for the slashing and has to be chilled,
                        // but we must avoid the over-slashing, ensure have the minimum active validators.
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
            .collect()
    }
}
