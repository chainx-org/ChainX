// Copyright 2018-2019 Chainpool.
//! Calculate the asset power for proposal 09.

/// global_distribution_ratio:
///
///    +---------------------+-----------------------------+
///    |                     |                             |
///    |                     |                             |
/// treasury(12)          airdrop(8)             cross_mining_and_staking(80) = 12:8:80 <== global_distribution_ratio
///                          |                             |
///                          |                             |
///                          |                    +--------+------+
///                          |                    |               |
///                  (airdrop1, share1)      cross_mining(1) :  staking(9) = 1:9 <== distribution_ratio
///                  (airdrop2, share2)           |
///                       ......                  |
///                                               |
///                                  (cross_chain_asset1, fixed_power1)
///                                  (cross_chain_asset2, fixed_power2)
///                                              ......
///
use super::*;

use xstaking::{OnDistributeAirdropAsset, OnDistributeCrossChainAsset};

impl<T: Trait> Module<T> {
    fn apply_precision(raw_power: u128, token: &Token) -> T::Balance {
        let token_asset =
            <xassets::Module<T>>::get_asset(token).expect("This token definitely exist.");
        let token_precision = 10_u64.pow(token_asset.precision().into());
        let power = raw_power as u64 * token_precision;
        power.into()
    }

    pub(crate) fn raw_airdrop_asset_power(token: &Token) -> Option<u128> {
        let (_t, a, cs) = xstaking::Module::<T>::global_distribution_ratio();

        let (cross_mining, staking) = xstaking::Module::<T>::distribution_ratio();
        let cross_mining_plus_staking = cross_mining + staking;
        let total_staked = xstaking::Module::<T>::total_staked();

        let total_airdrop_shares = <Self as OnDistributeAirdropAsset>::total_shares();
        let cur_airdrop_share = Self::airdrop_distribution_ratio_map(token);

        assert!(
            total_airdrop_shares > 0,
            "airdrop assets are non-empty and each airdrop asset share has to be > 0"
        );
        assert!(cs > 0, "CrossMiningAndPCXStaking shares > 0 is ensured in xstaking::set_global_distribution_ratio()");

        let total_token_balance = xassets::Module::<T>::all_type_total_asset_balance(token);

        if total_token_balance == 0u64.into() {
            return Some(0u64.into());
        }

        let power = u128::from(total_staked.into())
            * u128::from(cur_airdrop_share)
            * u128::from(a)
            * u128::from(cross_mining_plus_staking)
            / (u128::from(total_airdrop_shares)
                * u128::from(cs)
                * u128::from(staking)
                * u128::from(total_token_balance.into()));
        debug!(
            "[airdrop_asset_power]power({}) = total_staked({})*cur_airdrop_share({})*a({})*cross_mining_plus_staking({}) / (total_airdrop_shares({})*cs({})*staking({})*total_token_balance({}))",
            power, total_staked, cur_airdrop_share, a,cross_mining_plus_staking, total_airdrop_shares, cs, staking, total_token_balance
        );

        Some(power)
    }

    pub(crate) fn raw_cross_chain_asset_power(token: &Token) -> Option<u128> {
        let (cross_mining_shares, staking_shares) = xstaking::Module::<T>::distribution_ratio();

        let fixed_power = Self::fixed_cross_chain_asset_power_map(token);
        let (m1, m2) = xstaking::Module::<T>::collect_cross_mining_vs_staking(
            cross_mining_shares,
            staking_shares,
        );
        // Max 400
        if m1 <= m2 {
            debug!(
                "[raw_cross_chain_asset_power]m1{} <= m2{}, fixed_power:{}",
                m1, m2, fixed_power
            );
            Some(u128::from(fixed_power))
        } else {
            let total_staking_power = xstaking::Module::<T>::total_staked();

            let raw_total_cross_mining_power =
                <Self as OnDistributeCrossChainAsset>::total_cross_chain_mining_power();

            assert!(
                raw_total_cross_mining_power > 0,
                "cross chain assets are non-empty and each cross chain asset power has to be > 0"
            );

            assert!(
                staking_shares > 0,
                "staking_shares > 0 is ensured in xstaking::set_distribution_ratio()"
            );

            //   power_threshold
            // = total_staking_power * cross_mining_shares / staking_shares
            //
            //   power
            // = fixed_power * power_threshold / raw_total_cross_chain_mining_power
            let power = u128::from(fixed_power)
                * u128::from(total_staking_power.into())
                * u128::from(cross_mining_shares)
                / (raw_total_cross_mining_power * u128::from(staking_shares));

            Some(power)
        }
    }

    pub(super) fn asset_power_09(token: &Token) -> Option<T::Balance> {
        // One PCX one power.
        if <xassets::Module<T> as ChainT>::TOKEN == token.as_slice() {
            return Some(Self::one_pcx().into());
        }

        // airdrop assets
        if Self::is_airdrop_asset(token) {
            return Self::raw_airdrop_asset_power(token)
                .map(|raw_power| Self::apply_precision(raw_power, token));
        }

        // cross chain assets
        if Self::is_cross_chain_asset(token) {
            return Self::raw_cross_chain_asset_power(token)
                .map(|raw_power| Self::apply_precision(raw_power, token));
        }

        None
    }
}
