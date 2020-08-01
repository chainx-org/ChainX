use super::*;
use xp_mining_staking::SessionIndex;
use xpallet_support::debug;

mod proposal09;

impl<T: Trait> Module<T> {
    /// Simple u32 power of 2 function - simply uses a bit shift
    #[inline]
    fn pow2(n: u32) -> BalanceOf<T> {
        (1_u32 << n).saturated_into()
    }

    /// Returns the total reward for the session, assuming it ends with this block.
    ///
    /// Due to the migration of ChainX 1.0 to ChainX 2.0,
    fn this_session_reward(current_index: SessionIndex) -> BalanceOf<T> {
        let halving_epoch = current_index / SESSIONS_PER_ROUND;
        // FIXME: migration_offset
        let reward = INITIAL_REWARD.saturated_into::<BalanceOf<T>>() / Self::pow2(halving_epoch);
        reward
    }

    /// Issue new fresh PCX.
    #[inline]
    pub(crate) fn mint(receiver: &T::AccountId, value: BalanceOf<T>) {
        T::Currency::deposit_creating(receiver, value);
        Self::deposit_event(RawEvent::Mint(receiver.clone(), value));
    }

    /// Reward a (potential) validator by a specific amount.
    ///
    /// Add the reward to their balance, and their reward pot, pro-rata.
    fn apply_reward_validator(who: &T::AccountId, reward: BalanceOf<T>) {
        // Validator themselves can only directly gain 10%, the rest 90% is for the reward pot.
        let off_the_table = (reward.saturated_into() / 10).saturated_into();
        Self::mint(who, off_the_table);
        debug!("[mint]to validator {:?}: {:?}", who, off_the_table);

        // Issue the rest 90% to validator's reward pot.
        let to_reward_pot = reward - off_the_table;
        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(who);
        Self::mint(&reward_pot, to_reward_pot);
        debug!(
            "[mint]to the reward pot {:?}: {:?}",
            reward_pot, to_reward_pot
        );
    }

    /// Reward the intention and slash the validators that went offline in last session.
    ///
    /// If the slashed validator can't afford that penalty, it will be
    /// removed from the validator list.
    #[inline]
    fn reward_active_validator(validator: &T::AccountId, reward: BalanceOf<T>) {
        Self::apply_reward_validator(validator, reward);
    }

    /// 20% reward of each session is for the vesting schedule in the first halving epoch.
    fn try_vesting(current_index: SessionIndex, this_session_reward: BalanceOf<T>) -> BalanceOf<T> {
        // FIXME: consider the offset due to the migration.
        // SESSIONS_PER_ROUND --> offset
        if current_index < SESSIONS_PER_ROUND {
            let to_vesting = this_session_reward / 5.saturated_into();
            let vesting_account = Self::vesting_account();
            debug!(
                "[try_vesting]issue to the vesting account {:?}: {:?}",
                vesting_account, to_vesting
            );
            Self::mint(&vesting_account, to_vesting);
            this_session_reward - to_vesting
        } else {
            this_session_reward
        }
    }

    /// Distribute the session reward to all the receivers, returns the total reward for validators.
    pub(crate) fn distribute_session_reward(session_index: SessionIndex) -> BalanceOf<T> {
        let this_session_reward = Self::this_session_reward(session_index);

        let session_reward = Self::try_vesting(session_index, this_session_reward);

        Self::distribute_session_reward_impl_09(session_reward)
    }
}
