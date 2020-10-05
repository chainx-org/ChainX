// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use frame_support::{decl_module, decl_storage, traits::Currency};

#[cfg(feature = "std")]
use xpallet_mining_staking::{GenesisValidatorInfo, WeightType};
use xpallet_support::info;

pub type BalanceOf<T> = <<T as xpallet_assets::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

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
        config(balances): Vec<(T::AccountId, T::Balance)>;
        config(xbtc_assets): Vec<(T::AccountId, BalanceOf<T>)>;
        config(validators): Vec<GenesisValidatorInfo<T>>;
        config(nominators): Vec<(T::AccountId, Vec<(T::AccountId, xpallet_mining_staking::BalanceOf<T>, WeightType)>)>;
        config(unbonds): Vec<(T::AccountId, Vec<(T::AccountId, Vec<(xpallet_mining_staking::BalanceOf<T>, T::BlockNumber)>)>)>;
        config(xbtc_weight): WeightType;
        config(xbtc_miners): Vec<(T::AccountId, WeightType)>;
        build(|config| {
            use crate::genesis::{xassets, balances, xstaking, xminingasset};

            let now = std::time::Instant::now();

            balances::initialize::<T>(&config.balances);
            xassets::initialize::<T>(&config.xbtc_assets);
            xstaking::initialize::<T>(&config.validators, &config.nominators, &config.unbonds);
            xminingasset::initialize::<T>(config.xbtc_weight, &config.xbtc_miners);

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

        pub fn initialize<T: Trait>(balances: &[(T::AccountId, T::Balance)]) {
            for (who, free) in balances {
                T::AccountStore::insert(
                    who,
                    AccountData {
                        free: *free,
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub mod xassets {
        use crate::{BalanceOf, Trait};
        use xpallet_protocol::X_BTC;

        pub fn initialize<T: Trait>(xbtc_assets: &[(T::AccountId, BalanceOf<T>)]) {
            for (who, free) in xbtc_assets {
                xpallet_assets::Module::<T>::force_set_free_balance(&X_BTC, who, *free);
            }
        }
    }

    pub mod xstaking {
        use crate::Trait;
        use xpallet_mining_staking::{BalanceOf, GenesisValidatorInfo, WeightType};

        pub fn initialize<T: Trait>(
            validators: &[GenesisValidatorInfo<T>],
            nominators: &[(T::AccountId, Vec<(T::AccountId, BalanceOf<T>, WeightType)>)],
            unbonds: &[(
                T::AccountId,
                Vec<(T::AccountId, Vec<(BalanceOf<T>, T::BlockNumber)>)>,
            )],
        ) {
            /////// register validator
            let genesis_validators = validators.iter().map(|v| v.0.clone()).collect::<Vec<_>>();
            xpallet_mining_staking::Module::<T>::initialize_validators(validators)
                .expect("Failed to initialize staking validators");

            // 1. mock vote
            // 3. set vote weights
            for (nominator, nominations) in nominators {
                for (nominee, value, weight) in nominations {
                    // The dead validators in 1.0 has been dropped.
                    if genesis_validators.contains(nominee) {
                        // Validator self bonded already processed in initialize_validators()
                        if *nominee == *nominator {
                            continue;
                        }
                        xpallet_mining_staking::Module::<T>::force_bond(nominator, nominee, *value)
                            .expect("force bond failed");
                        xpallet_mining_staking::Module::<T>::force_set_nominator_vote_weight(
                            nominator, nominee, *weight,
                        );
                    }
                }
            }

            // 2. mock unbond
            for (nominator, unbonded_list) in unbonds {
                for (target, unbonded_chunks) in unbonded_list {
                    if genesis_validators.contains(target) {
                        for (value, locked_until) in unbonded_chunks {
                            xpallet_mining_staking::Module::<T>::force_unbond(
                                nominator,
                                target,
                                *value,
                                *locked_until,
                            )
                            .expect("force unbond failed");
                        }
                    }
                }
            }
        }
    }

    pub mod xminingasset {
        use crate::Trait;
        use xpallet_mining_staking::WeightType;
        use xpallet_protocol::X_BTC;

        pub fn initialize<T: Trait>(
            new_xbtc_weight: WeightType,
            xbtc_miners: &[(T::AccountId, WeightType)],
        ) {
            //////////    XAssets
            ////// Set mining weight.
            ////// 1. mining asset weight
            ////// 2. miner weight
            let current_block = frame_system::Module::<T>::block_number();
            for (miner, weight) in xbtc_miners {
                xpallet_mining_asset::Module::<T>::force_set_miner_mining_weight(
                    miner,
                    &X_BTC,
                    *weight,
                    current_block,
                );
            }
            xpallet_mining_asset::Module::<T>::force_set_asset_mining_weight(
                &X_BTC,
                new_xbtc_weight,
                current_block,
            );
        }
    }
}
