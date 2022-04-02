// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use sp_std::ops::Mul;
use sp_std::vec::Vec;

use super::*;

impl<T: Config> Pallet<T> {
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
        let calc_base_slash = |offender: &T::AccountId, slash_fraction: Perbill| {
            // https://github.com/paritytech/substrate/blob/c60f00840034017d4b7e6d20bd4fcf9a3f5b529a/frame/im-online/src/lib.rs#L773
            // slash_fraction is zero when <10% offline, in which case we still apply a minimum_penalty.
            if slash_fraction.is_zero() {
                minimum_penalty
            } else {
                let pot = Self::reward_pot_for(offender);
                slash_fraction.mul(Self::free_balance(&pot))
            }
        };

        let minimum_validator_count = Self::reasonable_minimum_validator_count() as usize;
        let mut active_count = Self::active_validator_set().count();
        let mut chill_offender_safe = |offender: T::AccountId| {
            // The offender does not have enough balance for the slashing and has to be chilled,
            // but we must avoid the over-slashing, ensure have the minimum active validators.
            if active_count > minimum_validator_count {
                Self::apply_force_chilled(&offender);
                active_count -= 1;
                Some(offender)
            } else {
                None
            }
        };

        offenders
            .into_iter()
            .flat_map(|(offender, slash_fraction)| {
                let base_slash = calc_base_slash(&offender, slash_fraction);
                let penalty = validator_rewards
                    .get(&offender)
                    .copied()
                    .map(|reward| reward + base_slash)
                    .unwrap_or(base_slash)
                    .max(minimum_penalty);
                match slasher.try_slash(&offender, penalty) {
                    SlashOutcome::Slashed(_) => {
                        debug!(
                            target: "runtime::mining::staking",
                            "Slash the offender:{:?} for penalty {:?} by the given slash_fraction:{:?} successfully",
                            offender, penalty, slash_fraction
                        );
                        None
                    }
                    SlashOutcome::InsufficientSlash(actual_slashed) => {
                        debug!(
                            target: "runtime::mining::staking",
                            "Insufficient reward pot balance of {:?}, actual slashed:{:?}",
                            offender, actual_slashed
                        );
                        chill_offender_safe(offender)
                    }
                    SlashOutcome::SlashFailed(e) => {
                        debug!(
                            target: "runtime::mining::staking",
                            "Slash the offender {:?} for {:?} somehow failed: {:?}", offender, penalty, e,
                        );
                        // we still chill the offender even the slashing failed as currently
                        // the offender is only the authorties without running a node.
                        //
                        // TODO: Reconsider this once https://github.com/paritytech/substrate/pull/7127
                        // is merged.
                        chill_offender_safe(offender)
                    }
                }
            })
            .collect()
    }
}
