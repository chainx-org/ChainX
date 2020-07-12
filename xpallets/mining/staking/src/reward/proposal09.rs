// Copyright 2018-2020 Chainpool.

//! New minted PCX distribution logic for ChainX Proposal 09.

use super::*;
use xpallet_support::debug;

impl<T: Trait> Module<T> {
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
                    "reward of per validator definitely less than u64::max_value()"
                );
                r.saturated_into::<T::Balance>()
            }
            None => panic!("stake * session_reward overflow!"),
        }
    }

    /// Calculates the individual reward according to the proportion and total reward.
    fn calc_individual_staking_reward(
        total_reward: T::Balance,
        my_stake: T::Balance,
        total_stake: T::Balance,
    ) -> T::Balance {
        let mine = my_stake.saturated_into::<u128>();
        let total = total_stake.saturated_into::<u128>();
        Self::generic_calculate_by_proportion(total_reward, mine, total)
    }

    fn calc_invididual_asset_mining_reward(
        total_reward: T::Balance,
        my_power: u128,
        total_power: u128,
    ) -> T::Balance {
        Self::generic_calculate_by_proportion(total_reward, my_power, total_power)
    }

    /// Distributes the invididual asset mining reward, returns the unpaid asset mining rewards.
    fn distribute_to_mining_assets(total_reward: T::Balance) -> T::Balance {
        let asset_mining_info = T::AssetMining::asset_mining_power();

        // [PASS*] No risk of sum overflow practically.
        //        u128::max_value() / u128::from(u64::max_value()) / u128::from(u32::max_value())
        //      = 4294967297 > u32::max_value() = 4294967295
        //        which means even we have u32::max_value() mining assets and each power of them
        //        is u32::max_value(), the computation of sum() here won't overflow.
        let mut total_power: u128 = asset_mining_info.iter().map(|(_, power)| power).sum();

        let mut total_reward = total_reward;

        for (asset_id, power) in asset_mining_info.into_iter() {
            if !total_power.is_zero() {
                let reward =
                    Self::calc_invididual_asset_mining_reward(total_reward, power, total_power);
                T::AssetMining::reward(asset_id, reward);
                total_power -= power;
                total_reward -= reward;
            }
        }

        total_reward
    }

    /// Reward to all the active validators pro rata.
    fn distribute_to_active_validators(session_reward: T::Balance) {
        let active_validators = Self::get_active_validator_set().collect::<Vec<_>>();
        let mut total_stake = active_validators
            .iter()
            .fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + *x);
        let mut total_reward = session_reward;
        for (validator, stake) in active_validators.into_iter() {
            // May become zero after meeting the last one.
            if !total_stake.is_zero() {
                let reward = Self::calc_individual_staking_reward(total_reward, stake, total_stake);
                Self::reward_active_validator(&validator, reward);
                total_stake -= stake;
                total_reward -= reward;
            }
        }
    }

    /// Issue new PCX to the action intentions and cross mining asset entities
    /// accroding to DistributionRatio.
    fn distribute_mining_rewards(total: T::Balance, treasury_account: &T::AccountId) -> T::Balance {
        let mining_distribution = Self::mining_distribution_ratio();
        let staking_reward = mining_distribution.calc_staking_reward::<T>(total);
        let max_asset_mining_reward = total - staking_reward;

        Self::distribute_to_active_validators(staking_reward);

        let real_asset_mining_reward = if let Some(treasury_extra) =
            mining_distribution.has_treasury_extra::<T>(max_asset_mining_reward)
        {
            Self::mint(treasury_account, treasury_extra);
            max_asset_mining_reward - treasury_extra
        } else {
            max_asset_mining_reward
        };

        let unpaid_asset_mining_reward =
            Self::distribute_to_mining_assets(real_asset_mining_reward);
        if !unpaid_asset_mining_reward.is_zero() {
            debug!(
                "[distribute_mining_rewards]unpaid_asset_mining_reward:{:?}",
                unpaid_asset_mining_reward
            );
            Self::mint(treasury_account, unpaid_asset_mining_reward);
        }

        staking_reward
    }

    pub(super) fn distribute_session_reward_impl_09(session_reward: T::Balance) -> T::Balance {
        let global_distribution = Self::global_distribution_ratio();
        let (treasury_reward, mining_reward) =
            global_distribution.calc_rewards::<T>(session_reward);

        // -> Treasury
        let treasury_account = T::TreasuryAccount::treasury_account();
        if !treasury_reward.is_zero() {
            Self::mint(&treasury_account, treasury_reward);
        }

        // -> Mining
        //      |-> XBTC(Asset Mining)
        //      |-> PCX(Staking)
        if !mining_reward.is_zero() {
            return Self::distribute_mining_rewards(mining_reward, &treasury_account);
        }

        Default::default()
    }
}
