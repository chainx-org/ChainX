// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

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

    /// Returns true if the time for first halving cycle has arrived.
    #[inline]
    fn first_halving_epoch_arrived(current_index: SessionIndex) -> bool {
        current_index > T::MigrationSessionOffset::get()
    }

    /// Returns the total reward for the session, assuming it ends with this block.
    pub(crate) fn this_session_reward(current_index: SessionIndex) -> BalanceOf<T> {
        let halving_epoch = if Self::first_halving_epoch_arrived(current_index) {
            (current_index - T::MigrationSessionOffset::get() - 1) / SESSIONS_PER_ROUND + 1
        } else {
            0
        };

        INITIAL_REWARD.saturated_into::<BalanceOf<T>>() / Self::pow2(halving_epoch)
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
        debug!("💸 Mint validator({:?}):{:?}", who, off_the_table);

        // Issue the rest 90% to validator's reward pot.
        let to_reward_pot = reward - off_the_table;
        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(who);
        Self::mint(&reward_pot, to_reward_pot);
        debug!("💸 Mint reward_pot({:?}):{:?}", reward_pot, to_reward_pot);
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
    pub(crate) fn try_vesting(
        current_index: SessionIndex,
        this_session_reward: BalanceOf<T>,
    ) -> BalanceOf<T> {
        if !Self::first_halving_epoch_arrived(current_index) {
            let to_vesting = this_session_reward / 5.saturated_into();
            let vesting_account = Self::vesting_account();
            Self::mint(&vesting_account, to_vesting);
            debug!("💸 Mint vesting({:?}):{:?}", vesting_account, to_vesting);
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
