use super::*;
use xp_mining_staking::SessionIndex;
use xpallet_support::debug;

mod proposal09;

impl<T: Trait> Module<T> {
    /// Simple u32 power of 2 function - simply uses a bit shift
    #[inline]
    fn pow2(n: u32) -> T::Balance {
        (1_u32 << n).saturated_into()
    }

    /// Returns the total reward for the session, assuming it ends with this block.
    ///
    /// Due to the migration of ChainX 1.0 to ChainX 2.0,
    fn this_session_reward(current_index: SessionIndex) -> T::Balance {
        let halving_epoch = current_index / SESSIONS_PER_ROUND;
        // FIXME: migration_offset
        let reward = INITIAL_REWARD.saturated_into::<T::Balance>() / Self::pow2(halving_epoch);
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
    fn apply_reward_validator(who: &T::AccountId, reward: T::Balance) {
        // Validator themselves can only directly gain 10%, the rest 90% is for the reward pot.
        let off_the_table = (reward.saturated_into() / 10).saturated_into();
        Self::mint(who, off_the_table);
        debug!("[reward_validator]issue to {:?}: {:?}", who, off_the_table);

        // Issue the rest 90% to validator's reward pot.
        let to_reward_pot = reward - off_the_table;
        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(who);
        Self::mint(&reward_pot, to_reward_pot);
        debug!(
            "[reward_validator]issue to the reward pot{:?}: {:?}",
            reward_pot, to_reward_pot
        );
    }

    /// Reward the intention and slash the validators that went offline in last session.
    ///
    /// If the slashed validator can't afford that penalty, it will be
    /// removed from the validator list.
    #[inline]
    fn reward_active_validator(validator: &T::AccountId, reward: T::Balance) {
        Self::apply_reward_validator(validator, reward);
        // FIXME: slash?
        // It the intention was an offline validator, we should enforce a slash.
        // if <MissedOfPerSession<T>>::exists(intention) {
        // Self::slash_active_offline_validator(intention, reward, validators);
        // }
    }

    /// Returns all the active validators as well as their total votes.
    ///
    /// Only these active validators are able to be rewarded on each new session,
    /// the inactive ones earn nothing.
    fn get_active_validator_set() -> impl Iterator<Item = (T::AccountId, T::Balance)> {
        Self::validator_set().filter(Self::is_active).map(|who| {
            let total_votes = Self::total_votes_of(&who);
            (who, total_votes)
        })
    }

    /// 20% reward of each session is for the vesting schedule in the first halving epoch.
    fn try_vesting(current_index: SessionIndex, this_session_reward: T::Balance) -> T::Balance {
        // FIXME: consider the offset due to the migration.
        // SESSIONS_PER_ROUND --> offset
        if current_index < SESSIONS_PER_ROUND {
            let to_vesting = this_session_reward / 5.saturated_into();
            debug!("[try_vesting] issue to the team: {:?}", to_vesting);
            Self::mint(&Self::vesting_account(), to_vesting);
            this_session_reward - to_vesting
        } else {
            this_session_reward
        }
    }

    /// Distribute the session reward to all the receivers.
    pub(crate) fn distribute_session_reward(session_index: SessionIndex) {
        let this_session_reward = Self::this_session_reward(session_index);

        let session_reward = Self::try_vesting(session_index, this_session_reward);

        Self::distribute_session_reward_impl_09(session_reward);
    }
}
