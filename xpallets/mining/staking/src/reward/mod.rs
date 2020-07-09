use super::*;
use xp_mining_staking::SessionIndex;
use xpallet_support::debug;

mod proposal09;

impl<T: Trait> Module<T> {
    /// Returns the total reward for the session, assuming it ends with this block.
    ///
    /// Due to the migration of ChainX 1.0 to ChainX 2.0,
    fn this_session_reward(current_index: SessionIndex) -> T::Balance {
        let halving_epoch = current_index / SESSIONS_PER_ROUND;
        // FIXME: migration_offset
        let reward = INITIAL_REWARD.saturated_into::<T::Balance>()
            / u32::pow(2, halving_epoch).saturated_into();
        reward
    }

    /// Issue new fresh PCX.
    #[inline]
    fn mint(receiver: &T::AccountId, value: T::Balance) {
        let _ = <xpallet_assets::Module<T>>::pcx_issue(receiver, value);
    }

    /// Reward a (potential) validator by a specific amount.
    ///
    /// Add the reward to their balance, and their reward pot, pro-rata.
    fn reward_validator(who: &T::AccountId, reward: T::Balance) {
        // Validator themselves can only directly gain 10%, the rest 90% is for the reward pot.
        let off_the_table = (reward.saturated_into() / 10).saturated_into();
        Self::mint(who, off_the_table);
        debug!("[reward_validator]issue to {:?}: {:?}", who, off_the_table);

        // Issue the rest 90% to validator's reward pot.
        let to_reward_pot = reward - off_the_table;
        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(who);
        Self::mint(&reward_pot, to_reward_pot);
        debug!(
            "[reward_validator] issue to the reward pot{:?}: {:?}",
            reward_pot, to_reward_pot
        );
    }

    /// Reward the intention and slash the validators that went offline in last session.
    ///
    /// If the slashed validator can't afford that penalty, it will be
    /// removed from the validator list.
    #[inline]
    fn reward_active_intention_and_try_slash(
        intention: &T::AccountId,
        reward: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) {
        Self::reward_validator(intention, reward);
        // It the intention was an offline validator, we should enforce a slash.
        // if <MissedOfPerSession<T>>::exists(intention) {
        // Self::slash_active_offline_validator(intention, reward, validators);
        // }
    }

    #[inline]
    fn generic_calculate_by_proportion<S: Into<u128>>(
        total_reward: T::Balance,
        mine: S,
        total: S,
    ) -> T::Balance {
        let mine: u128 = mine.saturated_into();
        let total: u128 = total.saturated_into();

        match mine.checked_mul(u128::from(total_reward.saturated_into())) {
            Some(x) => {
                let r = x / total;
                assert!(
                    r < u128::from(u64::max_value()),
                    "reward of per intention definitely less than u64::max_value()"
                );
                r.saturated_into::<T::Balance>()
            }
            None => panic!("stake * session_reward overflow!"),
        }
    }

    /// Calculate the individual reward according to the mining power of cross chain assets.
    fn multiply_by_mining_power(
        total_reward: T::Balance,
        my_power: u128,
        total_mining_power: u128,
    ) -> T::Balance {
        Self::generic_calculate_by_proportion(total_reward, my_power, total_mining_power)
    }

    // This is guarantee not to overflow on whatever values.
    // `num` must be inferior to `den` otherwise it will be reduce to `den`.
    pub fn multiply_by_rational(value: u64, num: u32, den: u32) -> u64 {
        let num = num.min(den);

        let result_divisor_part: u64 = value / u64::from(den) * u64::from(num);

        let result_remainder_part: u64 = {
            let rem: u64 = value % u64::from(den);

            // Fits into u32 because den is u32 and remainder < den
            let rem_u32 = rem.saturated_into::<u32>();

            // Multiplication fits into u64 as both term are u32
            let rem_part = u64::from(rem_u32) * u64::from(num) / u64::from(den);

            // Result fits into u32 as num < total_points
            (rem_part as u32).saturated_into()
        };

        result_divisor_part + result_remainder_part
    }

    /// Returns all the active validators as well as their total votes.
    ///
    /// Only these active validators are able to be rewarded on each new session,
    /// the inactive ones earn nothing.
    fn get_active_validator_set() -> impl Iterator<Item = (T::AccountId, T::Balance)> {
        Self::potential_validator_set()
            .filter(Self::is_active)
            .map(|who| {
                let total_votes = Self::total_votes_of(&who);
                (who, total_votes)
            })
    }

    /// 20% reward of each session is for the vesting schedule in the first halving epoch.
    fn try_apply_vesting(
        current_index: SessionIndex,
        this_session_reward: T::Balance,
    ) -> T::Balance {
        // FIXME: consider the offset due to the migration.
        // SESSIONS_PER_ROUND --> offset
        if current_index < SESSIONS_PER_ROUND {
            let to_vesting = this_session_reward / 5.saturated_into();
            debug!("[try_apply_vesting] issue to the team: {:?}", to_vesting);
            Self::mint(&Self::vesting_account(), to_vesting);
            this_session_reward - to_vesting
        } else {
            this_session_reward
        }
    }

    /// Distribute the session reward to all the receivers.
    pub(crate) fn distribute_session_reward(session_index: SessionIndex) {
        let this_session_reward = Self::this_session_reward(session_index);

        let session_reward = Self::try_apply_vesting(session_index, this_session_reward);

        Self::distribute_session_reward_impl_09(session_reward);
    }

    /// Calculates the individual reward according to the proportion and total reward.
    fn calc_reward_by_stake(
        total_reward: T::Balance,
        my_stake: T::Balance,
        total_stake: T::Balance,
    ) -> T::Balance {
        let mine = my_stake.saturated_into::<u128>();
        let total = total_stake.saturated_into::<u128>();
        Self::generic_calculate_by_proportion(total_reward, mine, total)
    }
}
