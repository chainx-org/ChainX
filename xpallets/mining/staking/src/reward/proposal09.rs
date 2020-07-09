// Copyright 2018-2020 Chainpool.

//! New minted PCX distribution logic for ChainX Proposal 09.

use super::*;
use sp_arithmetic::traits::BaseArithmetic;
use xpallet_support::debug;

impl<T: Trait> Module<T> {
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

    #[inline]
    fn multiply_by_shares(total_reward: T::Balance, share: u32, total_shares: u32) -> T::Balance {
        let reward =
            Self::multiply_by_rational(total_reward.saturated_into::<u64>(), share, total_shares);
        reward.saturated_into()
    }

    /// Proportional PCX allocation for the token assets.
    ///
    /// Return the value of unpaid reward.
    fn token_proportional_allocation<Entity, P: BaseArithmetic + Copy>(
        items: impl Iterator<Item = (Entity, P)>,
        total_shares: P,
        total_reward: T::Balance,
        reward_calculator: &dyn Fn(T::Balance, P, P) -> T::Balance,
        apply_reward: &dyn Fn(&Entity, T::Balance),
        log: &dyn Fn(&Entity, T::Balance),
    ) -> T::Balance {
        let mut total_reward = total_reward;
        let mut total_shares = total_shares;

        for (entity, share) in items {
            if !total_shares.is_zero() {
                let reward = reward_calculator(total_reward, share, total_shares);
                log(&entity, reward);
                apply_reward(&entity, reward);
                total_reward -= reward;
                total_shares -= share;
            }
        }

        total_reward
    }

    fn distribute_to_mining_assets(total_reward: T::Balance) -> T::Balance {
        todo!("reward x-type assets")
        /*
        let cross_chain_assets_info =
            T::OnDistributeCrossChainAsset::collect_cross_chain_assets_info();
        // [PASS*] No risk of sum overflow practically.
        //        u128::max_value() / u128::from(u64::max_value()) / u128::from(u32::max_value())
        //      = 4294967297 > u32::max_value() = 4294967295
        //        which means even we have u32::max_value() cross chain assets and each power of them
        //        is u32::max_value(), the computation of sum() here won't overflow.
        let total_mining_power: u128 = cross_chain_assets_info.iter().map(|(_, power)| power).sum();
        let logger = |_cross_mining_asset: &AssetId, _reward: T::Balance| {};

        Self::token_proportional_allocation(
            cross_chain_assets_info.into_iter(),
            total_mining_power,
            total_reward,
            &Self::multiply_by_mining_power,
            &T::OnReward::reward,
            &logger,
        )
        */
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
                let reward = Self::calc_reward_by_stake(total_reward, stake, total_stake);
                // Self::reward_active_intention_and_try_slash(&intention, reward);
                total_stake -= stake;
                total_reward -= reward;
            }
        }
    }

    /// Calculate the total staked PCX, i.e., total staking power.
    ///
    /// One (indivisible) PCX one power.
    #[inline]
    pub fn total_staked() -> T::Balance {
        Self::active_validator_votes().fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + x)
    }

    /// Issue new PCX to the action intentions and cross mining asset entities
    /// accroding to DistributionRatio.
    fn distribute_mining_rewards(total: T::Balance, treasury_account: &T::AccountId) {
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
                "[distribute_to_cross_mining_and_staking]unpaid_cross_mining_reward:{:?}",
                unpaid_asset_mining_reward
            );
            Self::mint(treasury_account, unpaid_asset_mining_reward);
        }
    }

    pub(super) fn distribute_session_reward_impl_09(session_reward: T::Balance) {
        let global_distribution = Self::global_distribution_ratio();
        let (treasury_reward, mining_reward) =
            global_distribution.calc_rewards::<T>(session_reward);

        // -> Treasury
        let treasury_account = T::GetTreasuryAccount::treasury_account();
        if !treasury_reward.is_zero() {
            Self::mint(&treasury_account, treasury_reward);
        }

        // -> Mining
        //      |-> XBTC(Asset Mining)
        //      |-> PCX(Staking)
        if !mining_reward.is_zero() {
            Self::distribute_mining_rewards(mining_reward, &treasury_account);
        }
    }
}
