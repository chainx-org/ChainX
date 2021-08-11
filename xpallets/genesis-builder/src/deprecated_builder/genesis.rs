//! This script was used when ChainX was migrated from 1.0 to 2.0.
//!
//! Although it is not reusable, it still can be seen as an example for other
//! similar regenesis processing, particularly on the parts about which states
//! we care about and how we initialize them on a brand new chain.

use crate::deprecated_builder::GenesisConfig;

mod balances {
    use frame_support::{traits::StoredMap, StorageValue};
    use pallet_balances::AccountData;
    use xp_genesis_builder::{BalancesParams, FreeBalanceInfo, WellknownAccounts};
    use xp_protocol::X_BTC;
    use xpallet_support::traits::TreasuryAccount;

    use crate::deprecated_builder::Trait;

    /// Returns the validator account by the given reward pot account.
    fn validator_for<'a, T: Trait, I: Iterator<Item = &'a (T::AccountId, T::AccountId)>>(
        target_pot: &T::AccountId,
        mut pots: I,
    ) -> Option<&'a T::AccountId> {
        pots.find(|(pot, _)| *pot == *target_pot)
            .map(|(_, validator)| validator)
    }

    pub fn initialize<T: Trait>(
        params: &BalancesParams<T::AccountId, T::Balance>,
        root_endowed: T::Balance,
        initial_authorities_endowed: T::Balance,
    ) {
        let BalancesParams {
            free_balances,
            wellknown_accounts,
        } = params;

        let WellknownAccounts {
            legacy_council,
            legacy_pots,
            legacy_xbtc_pot,
        } = wellknown_accounts;

        let set_free_balance = |who: &T::AccountId, free: &T::Balance| {
            T::AccountStore::insert(
                who,
                AccountData {
                    free: *free,
                    ..Default::default()
                },
            )
        };

        let treasury_account =
            <T as xpallet_mining_staking::Trait>::TreasuryAccount::treasury_account();

        let mut total_issuance = T::Balance::default();

        for FreeBalanceInfo { who, free } in free_balances {
            if *who == *legacy_council {
                let treasury_free = *free - root_endowed;
                set_free_balance(&treasury_account, &treasury_free);
            } else if *who == *legacy_xbtc_pot {
                let new_xbtc_pot = xpallet_mining_asset::Module::<T>::reward_pot_for(&X_BTC);
                set_free_balance(&new_xbtc_pot, free);
            } else if let Some(validator) = validator_for::<T, _>(who, legacy_pots.iter()) {
                let new_pot = xpallet_mining_staking::Module::<T>::reward_pot_for(validator);
                set_free_balance(&new_pot, free);
            } else {
                set_free_balance(who, free);
            }
            total_issuance += *free;
        }

        pallet_balances::TotalIssuance::<T>::mutate(|v| *v = total_issuance);
    }
}

mod xassets {
    use crate::{deprecated_builder::Trait, AssetBalanceOf};
    use xp_genesis_builder::FreeBalanceInfo;
    use xp_protocol::X_BTC;

    pub fn initialize<T: Trait>(xbtc_assets: &[FreeBalanceInfo<T::AccountId, AssetBalanceOf<T>>]) {
        for FreeBalanceInfo { who, free } in xbtc_assets {
            xpallet_assets::Module::<T>::force_set_free_balance(&X_BTC, who, *free);
        }
    }
}

mod xstaking {
    use crate::{deprecated_builder::Trait, StakingBalanceOf};
    use xp_genesis_builder::{Nomination, NominatorInfo, XStakingParams};

    pub fn initialize<T: Trait>(params: &XStakingParams<T::AccountId, StakingBalanceOf<T>>) {
        let XStakingParams {
            validators,
            nominators,
        } = params;

        let genesis_validators = validators.iter().map(|v| v.who.clone()).collect::<Vec<_>>();

        // Firstly register the genesis validators.
        xpallet_mining_staking::Module::<T>::initialize_legacy_validators(validators)
            .expect("Failed to initialize genesis staking validators");

        // Then mock the validator bond themselves and set the vote weights.
        for NominatorInfo {
            nominator,
            nominations,
        } in nominators
        {
            for Nomination {
                nominee,
                nomination,
                weight,
            } in nominations
            {
                // Not all `nominee` are in `genesis_validators` because the dead
                // validators in 1.0 have been dropped.
                if genesis_validators.contains(nominee) {
                    xpallet_mining_staking::Module::<T>::force_set_nominator_vote_weight(
                        nominator, nominee, *weight,
                    );
                    // Skip the validator self-bonding as it has already been processed
                    // in initialize_legacy_validators()
                    if *nominee == *nominator {
                        continue;
                    }
                    xpallet_mining_staking::Module::<T>::force_bond(
                        nominator,
                        nominee,
                        *nomination,
                    )
                    .expect("force validator self-bond can not fail; qed");
                }
            }
        }
    }
}

mod xmining_asset {
    use crate::deprecated_builder::Trait;
    use xp_genesis_builder::{XBtcMiner, XMiningAssetParams};
    use xp_protocol::X_BTC;

    /// Mining asset module initialization only involves the mining weight.
    /// - Set xbtc mining asset weight.
    /// - Set xbtc miners' weight.
    pub fn initialize<T: Trait>(params: &XMiningAssetParams<T::AccountId>) {
        let XMiningAssetParams {
            xbtc_miners,
            xbtc_info,
        } = params;
        let current_block = frame_system::Module::<T>::block_number();
        for XBtcMiner { who, weight } in xbtc_miners {
            xpallet_mining_asset::Module::<T>::force_set_miner_mining_weight(
                who,
                &X_BTC,
                *weight,
                current_block,
            );
        }
        xpallet_mining_asset::Module::<T>::force_set_asset_mining_weight(
            &X_BTC,
            xbtc_info.weight,
            current_block,
        );
    }
}

pub(crate) fn initialize<T: crate::deprecated_builder::Trait>(config: &GenesisConfig<T>) {
    let now = std::time::Instant::now();

    balances::initialize::<T>(
        &config.params.balances,
        config.root_endowed,
        config.initial_authorities_endowed,
    );
    xassets::initialize::<T>(&config.params.xassets);
    xstaking::initialize::<T>(&config.params.xstaking);
    xmining_asset::initialize::<T>(&config.params.xmining_asset);

    xp_logging::info!(
        "Took {:?}ms to orchestrate the exported state from ChainX 1.0",
        now.elapsed().as_millis()
    );
}
