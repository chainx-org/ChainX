// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use frame_support::{decl_module, decl_storage, traits::Currency};

#[cfg(feature = "std")]
use xp_genesis_builder::{BalancesParams, FreeBalanceInfo, XMiningAssetParams, XStakingParams};
use xpallet_support::info;

pub type BalanceOf<T> = <<T as xpallet_assets::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

type StakingBalanceOf<T> = xpallet_mining_staking::BalanceOf<T>;

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
        config(balances): BalancesParams<T::AccountId, T::Balance>;
        config(xbtc_assets): Vec<FreeBalanceInfo<T::AccountId, BalanceOf<T>>>;
        config(xstaking): XStakingParams<T::AccountId, StakingBalanceOf<T>>;
        config(xmining_asset): XMiningAssetParams<T::AccountId>;
        build(|config| {
            use crate::genesis::{xassets, balances, xstaking, xminingasset};

            let now = std::time::Instant::now();

            balances::initialize::<T>(&config.balances);
            xassets::initialize::<T>(&config.xbtc_assets);
            xstaking::initialize::<T>(&config.xstaking);
            xminingasset::initialize::<T>(&config.xmining_asset);

            info!(
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
        use frame_support::traits::StoredMap;
        use pallet_balances::AccountData;
        use xp_genesis_builder::{BalancesParams, FreeBalanceInfo, WellknownAccounts};
        use xpallet_mining_staking::RewardPotAccountFor;
        use xpallet_support::traits::TreasuryAccount;

        fn validator_for<'a, T: Trait, I: Iterator<Item = &'a (T::AccountId, T::AccountId)>>(
            target: &T::AccountId,
            mut pots: I,
        ) -> Option<&'a T::AccountId> {
            pots.find(|(pot, _)| *pot == *target).map(|(_, v)| v)
        }

        pub fn initialize<T: Trait>(params: &BalancesParams<T::AccountId, T::Balance>) {
            let BalancesParams {
                free_balances,
                wellknown_accounts,
            } = params;

            let WellknownAccounts {
                legacy_council,
                legacy_team,
                legacy_pots,
            } = wellknown_accounts;

            let treasury_account =
                <T as xpallet_mining_staking::Trait>::TreasuryAccount::treasury_account();

            let set_free_balance = |who: &T::AccountId, free: &T::Balance| {
                T::AccountStore::insert(
                    who,
                    AccountData {
                        free: *free,
                        ..Default::default()
                    },
                )
            };

            for FreeBalanceInfo { who, free } in free_balances {
                if *who == *legacy_council {
                    set_free_balance(&treasury_account, free);
                } else if *who == *legacy_team {
                    set_free_balance(
                        &xpallet_mining_staking::Module::<T>::vesting_account(),
                        free,
                    );
                } else if let Some(validator) = validator_for::<T, _>(who, legacy_pots.iter()) {
                    let new_pot = <T as xpallet_mining_staking::Trait>::DetermineRewardPotAccount::reward_pot_account_for(
                            validator,
                        );
                    set_free_balance(&new_pot, free);
                } else {
                    set_free_balance(who, free);
                }
            }
        }
    }

    pub mod xassets {
        use crate::{BalanceOf, Trait};
        use xp_genesis_builder::FreeBalanceInfo;
        use xpallet_protocol::X_BTC;

        pub fn initialize<T: Trait>(xbtc_assets: &[FreeBalanceInfo<T::AccountId, BalanceOf<T>>]) {
            for FreeBalanceInfo { who, free } in xbtc_assets {
                xpallet_assets::Module::<T>::force_set_free_balance(&X_BTC, who, *free);
            }
        }
    }

    pub mod xstaking {
        use crate::Trait;
        use xp_genesis_builder::{Nomination, NominatorInfo, XStakingParams};
        use xpallet_mining_staking::BalanceOf;

        pub fn initialize<T: Trait>(params: &XStakingParams<T::AccountId, BalanceOf<T>>) {
            let XStakingParams {
                validators,
                nominators,
            } = params;

            let genesis_validators = validators.iter().map(|v| v.who.clone()).collect::<Vec<_>>();

            // register validator
            xpallet_mining_staking::Module::<T>::initialize_validators(validators)
                .expect("Failed to initialize staking validators");

            // 1. mock vote
            // 3. set vote weights
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
                    // The dead validators in 1.0 has been dropped.
                    if genesis_validators.contains(nominee) {
                        // Validator self bonded already processed in initialize_validators()
                        if *nominee == *nominator {
                            continue;
                        }
                        xpallet_mining_staking::Module::<T>::force_bond(
                            nominator,
                            nominee,
                            *nomination,
                        )
                        .expect("force bond failed");
                        xpallet_mining_staking::Module::<T>::force_set_nominator_vote_weight(
                            nominator, nominee, *weight,
                        );
                    }
                }
            }
        }
    }

    pub mod xminingasset {
        use crate::Trait;
        use xp_genesis_builder::{XMiningAssetParams, XbtcMiner};
        use xpallet_protocol::X_BTC;

        /// Set mining weight.
        /// 1. mining asset weight
        /// 2. miner weight
        pub fn initialize<T: Trait>(params: &XMiningAssetParams<T::AccountId>) {
            let XMiningAssetParams {
                xbtc_miners,
                xbtc_info,
            } = params;
            let current_block = frame_system::Module::<T>::block_number();
            for XbtcMiner { who, weight } in xbtc_miners {
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
