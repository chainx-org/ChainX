// Copyright 2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use frame_support::{decl_module, decl_storage};

#[cfg(feature = "std")]
use xp_genesis_builder::AllParams;
#[cfg(feature = "std")]
use xpallet_assets::BalanceOf as AssetBalanceOf;
#[cfg(feature = "std")]
use xpallet_mining_staking::BalanceOf as StakingBalanceOf;

pub trait Trait:
    pallet_balances::Trait + xpallet_mining_asset::Trait + xpallet_mining_staking::Trait
{
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

decl_storage! {
    trait Store for Module<T: Trait> as XGenesisBuilder {}
    add_extra_genesis {
        config(params): AllParams<T::AccountId, T::Balance, AssetBalanceOf<T>, StakingBalanceOf<T>>;
        config(total_endowed): T::Balance;
        build(|config| {
            use crate::genesis::{xassets, balances, xstaking, xmining_asset};

            let now = std::time::Instant::now();

            balances::initialize::<T>(&config.params.balances, config.total_endowed);
            xassets::initialize::<T>(&config.params.xassets);
            xstaking::initialize::<T>(&config.params.xstaking);
            xmining_asset::initialize::<T>(&config.params.xmining_asset);

            xp_logging::info!(
                "Took {:?}ms to orchestrate the exported state from ChainX 1.0",
                now.elapsed().as_millis()
            );
        })
    }
}

#[cfg(feature = "std")]
mod genesis {
    pub mod balances {
        use crate::Trait;
        use frame_support::{sp_runtime::traits::Saturating, traits::StoredMap, StorageValue};
        use pallet_balances::AccountData;
        use xp_genesis_builder::{BalancesParams, FreeBalanceInfo, WellknownAccounts};
        use xpallet_support::traits::TreasuryAccount;

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
            total_endowed: T::Balance,
        ) {
            let BalancesParams {
                free_balances,
                wellknown_accounts,
            } = params;

            let WellknownAccounts {
                legacy_council,
                legacy_team,
                legacy_pots,
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

            let vesting_account = xpallet_mining_staking::Module::<T>::vesting_account();

            let mut issuance = T::Balance::default();

            for FreeBalanceInfo { who, free } in free_balances {
                if *who == *legacy_council {
                    set_free_balance(&treasury_account, free);
                } else if *who == *legacy_team {
                    let vesting_free = *free - total_endowed;
                    set_free_balance(&vesting_account, &vesting_free);
                } else if let Some(validator) = validator_for::<T, _>(who, legacy_pots.iter()) {
                    let new_pot = xpallet_mining_staking::Module::<T>::reward_pot_for(validator);
                    set_free_balance(&new_pot, free);
                } else {
                    set_free_balance(who, free);
                }
                issuance += *free;
            }

            pallet_balances::TotalIssuance::<T>::mutate(|v| *v = v.saturating_add(issuance));
        }
    }

    pub mod xassets {
        use crate::{AssetBalanceOf, Trait};
        use xp_genesis_builder::FreeBalanceInfo;
        use xp_protocol::X_BTC;

        pub fn initialize<T: Trait>(
            xbtc_assets: &[FreeBalanceInfo<T::AccountId, AssetBalanceOf<T>>],
        ) {
            for FreeBalanceInfo { who, free } in xbtc_assets {
                xpallet_assets::Module::<T>::force_set_free_balance(&X_BTC, who, *free);
            }
        }
    }

    pub mod xstaking {
        use crate::{StakingBalanceOf, Trait};
        use xp_genesis_builder::{Nomination, NominatorInfo, XStakingParams};

        pub fn initialize<T: Trait>(params: &XStakingParams<T::AccountId, StakingBalanceOf<T>>) {
            let XStakingParams {
                validators,
                nominators,
            } = params;

            let genesis_validators = validators.iter().map(|v| v.who.clone()).collect::<Vec<_>>();

            // Firstly register the genesis validators.
            xpallet_mining_staking::Module::<T>::initialize_validators(validators)
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
                        // in initialize_validators()
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

    pub mod xmining_asset {
        use crate::Trait;
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
}
