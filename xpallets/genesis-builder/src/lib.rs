// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use frame_support::{decl_module, decl_storage, traits::Currency};

#[cfg(feature = "std")]
use xpallet_mining_staking::GenesisValidatorInfo;
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
        config(xassets): Vec<(T::AccountId, BalanceOf<T>)>;
        config(validators): Vec<GenesisValidatorInfo<T>>;
        build(|config| {
            use crate::genesis::{xassets, balances, xstaking};

            let now = std::time::Instant::now();

            balances::initialize::<T>(&config.balances);
            xassets::initialize::<T>(&config.xassets);
            xstaking::initialize::<T>(&config.validators);

            info!("Took {:?}ms to orchestrate the exported state from ChainX 1.0", now.elapsed().as_millis());

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

        pub fn initialize<T: Trait>(btc_assets: &[(T::AccountId, BalanceOf<T>)]) {
            for (who, free) in btc_assets {
                xpallet_assets::Module::<T>::force_set_free_balance(&X_BTC, who, *free);
            }
        }
    }

    pub mod xstaking {
        use crate::Trait;
        use xpallet_mining_staking::GenesisValidatorInfo;

        pub fn initialize<T: Trait>(validators: &[GenesisValidatorInfo<T>]) {
            /////// register validator
            xpallet_mining_staking::Module::<T>::register_genesis_validators(validators)
                .expect("Failed to register genesis validators");

            //////// Nominator
            // TODO:
            // 1. mock vote
            // 2. mock unbond
            // 3. set vote weights

            //////////    XAssets
        }
    }

    pub mod xminingasset {
        use crate::Trait;

        pub fn initialize<T: Trait>(balances: Vec<(T::AccountId, T::Balance)>) {
            //////////    XAssets
            ////// Set mining weight.
            ////// 1. mining asset weight
            ////// 2. miner weight
        }
    }
}
