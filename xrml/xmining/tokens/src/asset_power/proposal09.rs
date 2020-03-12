// Copyright 2018-2019 Chainpool.
//! Calculate the asset power for proposal 09.

use super::*;

use xstaking::{OnDistributeAirdropAsset, OnDistributeCrossChainAsset};

impl<T: Trait> Module<T> {
    pub(crate) fn airdrop_asset_power(token: &Token) -> Option<T::Balance> {
        let (_t, a, cs) = xstaking::Module::<T>::global_distribution_ratio();
        let total_staked = xstaking::Module::<T>::total_staked();

        let total_airdrop_shares = <Self as OnDistributeAirdropAsset>::total_shares();
        let cur_airdrop_share = Self::airdrop_distribution_ratio_map(token);

        assert!(
            total_airdrop_shares > 0,
            "airdrop assets are non-empty and each airdrop asset share has to be > 0"
        );
        assert!(cs > 0, "CrossMiningAndPCXStaking shares > 0 is ensured in xstaking::set_global_distribution_ratio()");

        let airdrop_total_power =
            u128::from(total_staked.into()) * u128::from(cur_airdrop_share) * u128::from(a)
                / (u128::from(total_airdrop_shares) * u128::from(cs));

        let total_token_balance = xassets::Module::<T>::all_type_total_asset_balance(token);

        if total_token_balance == 0u64.into() {
            return Some(0u64.into());
        }

        let power = airdrop_total_power / u128::from(total_token_balance.into());

        Some((power as u64).into())
    }

    pub(crate) fn cross_chain_asset_power(token: &Token) -> Option<T::Balance> {
        let (cross_mining_shares, staking_shares) = xstaking::Module::<T>::distribution_ratio();
        let fixed_power = Self::fixed_cross_chain_asset_power_map(token);
        let (m1, m2) = xstaking::Module::<T>::collect_cross_mining_vs_staking(
            cross_mining_shares,
            staking_shares,
        );
        // Max 400
        if m1 <= m2 {
            Some(u64::from(fixed_power).into())
        } else {
            let total_staking_power = xstaking::Module::<T>::total_staked();

            assert!(
                staking_shares > 0,
                "staking_shares > 0 is ensured in xstaking::set_distribution_ratio()"
            );

            let power_threshold = u128::from(total_staking_power.into())
                * u128::from(cross_mining_shares)
                / u128::from(staking_shares);

            let raw_total_cross_mining_power =
                <Self as OnDistributeCrossChainAsset>::total_cross_chain_mining_power();

            assert!(
                raw_total_cross_mining_power > 0,
                "cross chain assets are non-empty and each cross chain asset power has to be > 0"
            );

            let power = u128::from(fixed_power) * power_threshold / raw_total_cross_mining_power;

            Some((power as u64).into())
        }
    }

    pub(super) fn asset_power_09(token: &Token) -> Option<T::Balance> {
        // One PCX one power.
        if <xassets::Module<T> as ChainT>::TOKEN == token.as_slice() {
            return Some(Self::one_pcx().into());
        }

        // airdrop assets
        if Self::is_airdrop_asset(token) {
            return Self::airdrop_asset_power(token);
        }

        // cross chain assets
        if Self::is_cross_chain_asset(token) {
            return Self::cross_chain_asset_power(token);
        }

        None
    }
}
