// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

//! Minimal Pallet that stores the numeric Ethereum-style chain id in the runtime.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet, traits::Get, weights::Weight};

pub use pallet::*;

#[pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// The Ethereum Chain Id Pallet
    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    /// Configuration trait of this pallet.
    #[pallet::config]
    pub trait Config: frame_system::Config {}

    impl<T: Config> Get<u64> for Pallet<T> {
        fn get() -> u64 {
            Self::chain_id()
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn chain_id)]
    pub type ChainId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub chain_id: u64,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self { chain_id: 1501u64 }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            ChainId::<T>::put(self.chain_id);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(100_000_000u64)]
        pub fn set_chain_id(
            origin: OriginFor<T>,
            #[pallet::compact] new_chain_id: u64,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let _ = Self::set_chain_id_inner(new_chain_id);

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn set_chain_id_inner(new_chain_id: u64) -> Weight {
        ChainId::<T>::mutate(|chain_id| *chain_id = new_chain_id);
        T::DbWeight::get().write
    }
}
