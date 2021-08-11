// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;
#[allow(unused_imports)]
use micromath::F32Ext;
use xp_logging::debug;
mod proposal09;

impl<T: Trait> Module<T> {
    /// Simple u32 power of 2 function - simply uses a bit shift
    #[inline]
    fn pow2(n: u32) -> BalanceOf<T> {
        (1_u32 << n).saturated_into()
    }

    /// (1/2)^(n+1) < (2100 - x) / 2100 <= (1/2)^n
    /// Returns the total reward for the session, assuming it ends with this block.
    pub(crate) fn this_session_reward() -> BalanceOf<T> {
        let total_issuance = T::Currency::total_issuance().saturated_into::<u64>() as f64; // x
        let fixed_total = FIXED_TOTAL as f64;
        let tt = (fixed_total / (fixed_total - total_issuance)) as f32;
        let halving_epoch = tt.log2().trunc() as u32; // n

        INITIAL_REWARD.saturated_into::<BalanceOf<T>>() / Self::pow2(halving_epoch)
    }

    /// Issue new fresh PCX.
    #[inline]
    pub(crate) fn mint(receiver: &T::AccountId, value: BalanceOf<T>) {
        T::Currency::deposit_creating(receiver, value);
        Self::deposit_event(Event::<T>::Minted(receiver.clone(), value));
    }

    /// Reward a (potential) validator by a specific amount.
    ///
    /// Add the reward to their balance, and their reward pot, pro-rata.
    fn apply_reward_validator(who: &T::AccountId, reward: BalanceOf<T>) {
        // Validator themselves can only directly gain 10%, the rest 90% is for the reward pot.
        let off_the_table = reward.saturated_into::<BalanceOf<T>>() / 10u32.saturated_into();
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

    /// Distribute the session reward to all the receivers, returns the total reward for validators.
    pub(crate) fn distribute_session_reward() -> Vec<(T::AccountId, BalanceOf<T>)> {
        let session_reward = Self::this_session_reward(/*session_index*/);

        Self::distribute_session_reward_impl_09(session_reward)
    }
}
