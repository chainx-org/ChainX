// Copyright 2018-2020 Chainpool.
//! New minted PCX distribution logic for ChainX Proposal 09.

use super::*;
#[cfg(feature = "std")]
use xsupport::u8array_to_string;

impl<T: Trait> Module<T> {
    /// Calculate the top level distribution of each session reward
    /// without the potential team funding.
    pub fn calc_global_distribution(
        session_reward: T::Balance,
    ) -> (T::Balance, T::Balance, T::Balance) {
        let (t_shares, a_shares, cs_shares) = Self::global_distribution_ratio();
        let total_shares = t_shares + a_shares + cs_shares;
        // [PASS] Division by zero check.
        //        total_shares > 0 is ensured set_global_distribution_ratio().
        let for_treasury =
            Self::multiply_by_rational(session_reward.into(), t_shares, total_shares).into();
        let for_airdrop =
            Self::multiply_by_rational(session_reward.into(), a_shares, total_shares).into();
        let for_cross_mining_and_staking = session_reward - for_treasury - for_airdrop;
        (for_treasury, for_airdrop, for_cross_mining_and_staking)
    }

    #[inline]
    fn multiply_by_shares(total_reward: T::Balance, share: u32, total_shares: u32) -> T::Balance {
        let reward = Self::multiply_by_rational(total_reward.into(), share, total_shares);
        reward.into()
    }

    /// Proportional PCX allocation for the token assets.
    ///
    /// Return the value of unpaid reward.
    fn token_proportional_allocation<Entity, P: primitives::traits::SimpleArithmetic + Copy>(
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

    /// Distribution for airdrop assets are fixed and dependent on AirdropDistributionRatioMap only.
    fn distribute_to_airdrop_assets(total_reward: T::Balance) -> T::Balance {
        let airdrop_assets_info = T::OnDistributeAirdropAsset::collect_airdrop_assets_info();
        // [PASS] airdrop_assets_info.sum() overflow check.
        //        sum of airdrop asset shares won't exceed u32::max_value(), ensured in tokens::set_airdrop_distribution_ratio().
        //
        // [PASS] Division by zero check.
        //        total_shares > 0 is ensured in tokens::set_airdrop_distribution_ratio().
        let total_shares = airdrop_assets_info.iter().map(|(_, share)| share).sum();
        let logger = |_airdrop_asset: &Token, _reward: T::Balance| {
            debug!(
                "[distribute_to_airdrop_assets]token:{}, reward:{}",
                u8array_to_string(_airdrop_asset),
                _reward
            )
        };

        Self::token_proportional_allocation(
            airdrop_assets_info.into_iter(),
            total_shares,
            total_reward,
            &Self::multiply_by_shares,
            &T::OnReward::reward,
            &logger,
        )
    }

    fn distribute_to_cross_chain_assets(total_reward: T::Balance) -> T::Balance {
        let cross_chain_assets_info =
            T::OnDistributeCrossChainAsset::collect_cross_chain_assets_info();
        // [PASS*] No risk of sum overflow practically.
        //        u128::max_value() / u128::from(u64::max_value()) / u128::from(u32::max_value())
        //      = 4294967297 > u32::max_value() = 4294967295
        //        which means even we have u32::max_value() cross chain assets and each power of them
        //        is u32::max_value(), the computation of sum() here won't overflow.
        let total_mining_power: u128 = cross_chain_assets_info.iter().map(|(_, power)| power).sum();
        let logger = |_cross_mining_asset: &Token, _reward: T::Balance| {
            debug!(
                "[distribute_to_cross_chain_assets]token:{}, reward:{}",
                u8array_to_string(_cross_mining_asset),
                _reward
            )
        };

        Self::token_proportional_allocation(
            cross_chain_assets_info.into_iter(),
            total_mining_power,
            total_reward,
            &Self::multiply_by_mining_power,
            &T::OnReward::reward,
            &logger,
        )
    }

    /// Reward to all the intentions pro rata.
    fn distribute_to_active_intentions(
        total_reward: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) {
        let active_intentions = Self::get_active_intentions_info().collect::<Vec<_>>();
        let mut total_stake = active_intentions
            .iter()
            .fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + *x);
        let mut total_reward = total_reward;
        for (intention, stake) in active_intentions.into_iter() {
            // May become zero after meeting the last one.
            if !total_stake.is_zero() {
                let reward = Self::calculate_reward_by_stake(total_reward, stake, total_stake);
                Self::reward_active_intention_and_try_slash(&intention, reward, validators);
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
        Self::get_active_intentions_info().fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + x)
    }

    /// Return a tuple (m1, m2) for comparing whether cross_mining_power are reaching the upper limit.
    ///
    /// If m1 >= m2, the cross mining cap has reached, all the reward calculated by the shares go to
    /// the cross chain assets, but its unit mining power starts to decrease.
    pub fn collect_cross_mining_vs_staking(
        cross_mining_shares: u32,
        staking_shares: u32,
    ) -> (u128, u128) {
        let total_staking_power = Self::total_staked();
        let total_cross_mining_power =
            T::OnDistributeCrossChainAsset::total_cross_chain_mining_power();
        // When:
        //
        //  total_cross_mining_power     1(cross_mining_shares)
        //  ------------------------ >= -----------------------
        //        total_stake            9(staking_shares)
        //
        // there is no extra treasury split.
        //
        // Otherwise the difference will be distruted to the council_account again.
        let m1 = total_cross_mining_power * u128::from(staking_shares);
        let m2 = u128::from(total_staking_power.into()) * u128::from(cross_mining_shares);
        debug!("[collect_cross_mining_vs_staking]m1=total_cross_mining_power({})*staking_shares({}), m2=total_staking_power({})*cross_mining_shares({})", total_cross_mining_power, staking_shares, total_staking_power, cross_mining_shares);
        (m1, m2)
    }

    /// Split out an extra treasury reward from cross chain mining's
    /// if the mining power of cross chain assets is less than the threshold.
    fn try_split_extra_treasury(
        cross_mining_reward_cap: T::Balance,
        cross_mining_shares: u32,
        staking_shares: u32,
    ) -> T::Balance {
        let (m1, m2) = Self::collect_cross_mining_vs_staking(cross_mining_shares, staking_shares);
        if m1 >= m2 {
            debug!(
                "[try_split_extra_treasury] m1({}) >= m2({}), no extra treasury split.",
                m1, m2
            );
            cross_mining_reward_cap
        } else {
            assert!(
                m2 > 0,
                "cross_mining_shares is ensured to be positive in set_distribution_ratio()"
            );
            // There could be some computation loss here, but it's ok.
            let extra_treasury = (m2 - m1) * u128::from(cross_mining_reward_cap.into()) / m2;
            let extra_treasury: T::Balance = (extra_treasury as u64).into();
            if !extra_treasury.is_zero() {
                Self::distribute_to_treasury(extra_treasury);
            }
            debug!(
                "[try_split_extra_treasury](m2({}) - m1({})) * {} / {} = extra_treasury({})",
                m2, m1, cross_mining_reward_cap, m2, extra_treasury
            );
            cross_mining_reward_cap - extra_treasury
        }
    }

    /// Issue new PCX to the action intentions and cross mining asset entities
    /// accroding to DistributionRatio.
    fn distribute_to_cross_mining_and_staking(
        total: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) {
        let (cross_mining_shares, staking_shares) = Self::distribution_ratio();

        // The amount of new minted PCX for the staking intentions is fixed and
        // only determined by DistributionRatio.
        let for_staking = Self::multiply_by_rational(
            total.into(),
            staking_shares,
            cross_mining_shares + staking_shares,
        )
        .into();
        Self::distribute_to_active_intentions(for_staking, validators);

        // Cross chain assets with possible extra treasury.
        let for_cross_mining_cap = total - for_staking;
        let for_cross_mining = Self::try_split_extra_treasury(
            for_cross_mining_cap,
            cross_mining_shares,
            staking_shares,
        );
        let unpaid_cross_mining_reward = Self::distribute_to_cross_chain_assets(for_cross_mining);
        if !unpaid_cross_mining_reward.is_zero() {
            debug!(
                "[distribute_to_cross_mining_and_staking]unpaid_cross_mining_reward:{}",
                unpaid_cross_mining_reward
            );
            Self::distribute_to_treasury(unpaid_cross_mining_reward);
        }
    }

    /// Issue new PCX given the amount to the council account.
    #[inline]
    fn distribute_to_treasury(value: T::Balance) {
        let council_account = xaccounts::Module::<T>::council_account();
        debug!(
            "[distribute_to_treasury]council_account: {}, value: {}",
            council_account, value
        );
        Self::mint(&council_account, value);
    }

    /// Superseded by Proposal 12.
    #[allow(unused)]
    fn airdrop_proposal09(for_airdrop: T::Balance) {
        let unpaid_airdrop_reward = Self::distribute_to_airdrop_assets(for_airdrop);
        if !unpaid_airdrop_reward.is_zero() {
            debug!(
                "[distribute_session_reward_impl_09]unpaid_airdrop_reward:{}",
                unpaid_airdrop_reward
            );
            Self::distribute_to_treasury(unpaid_airdrop_reward);
        }
    }

    /// SDOT, LBTC's reward will be distributed to the treasury according to ChainX Proposal 12.
    /// At this time, SDOT+LBTC = all airdrop assets, therefore all the airdrop reward
    /// goes to the treasury.
    fn airdrop_proposal12(for_airdrop: T::Balance) {
        Self::distribute_to_treasury(for_airdrop);
    }

    pub(super) fn distribute_session_reward_impl_09(
        session_reward: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) {
        let (for_treasury, for_airdrop, for_cross_mining_and_staking) =
            Self::calc_global_distribution(session_reward);

        // treasury -> CouncilAccount
        if !for_treasury.is_zero() {
            Self::distribute_to_treasury(for_treasury);
        }

        // airdrop -> SDOT, LBTC
        // The unpaid airdrop reward happens when there is no airdrop asset.
        if !for_airdrop.is_zero() {
            Self::airdrop_proposal12(for_airdrop);
        }

        // cross_mining_and_staking -> XBTC, PCX
        if !for_cross_mining_and_staking.is_zero() {
            Self::distribute_to_cross_mining_and_staking(for_cross_mining_and_staking, validators);
        }
    }
}
