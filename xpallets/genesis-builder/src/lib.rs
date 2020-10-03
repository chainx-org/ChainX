// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use frame_support::{decl_module, decl_storage, traits::Currency};

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
        build(|config| {
            use crate::genesis::balances;

            balances::initialize::<T>(&config.balances);
        })
    }
}

#[cfg(feature = "std")]
mod genesis {
    pub mod balances {
        use frame_support::traits::StoredMap;
        use pallet_balances::AccountData;

        pub fn initialize<T: crate::Trait>(balances: &[(T::AccountId, T::Balance)]) {
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
        pub fn initialize<T: crate::Trait>(btc_assets: &[(T::AccountId, crate::BalanceOf<T>)]) {
            for (who, free) in btc_assets {
                xpallet_assets::Module::<T>::issue(&xpallet_protocol::X_BTC, who, *free)
                    .expect("Failed to issue BTC asset");
            }
        }
    }

    pub mod xstaking {
        pub fn initialize<T: crate::Trait>(balances: Vec<(T::AccountId, T::Balance)>) {
            //////////     XStaking
            /////// register validator
            //
            // TODO:
            // 1. mock vote
            // 2. mock unbond
            // 3. set vote weights

            //////////    XAssets
        }
    }

    pub mod xminingasset {
        pub fn initialize<T: crate::Trait>(balances: Vec<(T::AccountId, T::Balance)>) {
            //////////    XAssets
            ////// Set mining weight.
            ////// 1. mining asset weight
            ////// 2. miner weight
        }
    }
}
