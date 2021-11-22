// Copyright 2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use xp_genesis_builder::AllParams;
#[cfg(feature = "std")]
use xpallet_assets::BalanceOf as AssetBalanceOf;
#[cfg(feature = "std")]
use xpallet_mining_staking::BalanceOf as StakingBalanceOf;

#[cfg(feature = "std")]
mod regenesis;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use sp_std::marker::PhantomData;

    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_balances::Config
        + xpallet_mining_asset::Config
        + xpallet_mining_staking::Config
    {
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub params: AllParams<T::AccountId, T::Balance, AssetBalanceOf<T>, StakingBalanceOf<T>>,
        pub initial_authorities: Vec<Vec<u8>>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                params: Default::default(),
                initial_authorities: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    #[cfg(feature = "std")]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            regenesis::initialize(&self)
        }
    }
}
