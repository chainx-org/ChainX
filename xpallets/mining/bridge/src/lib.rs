// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! # Asset Mining Module

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]

mod impls;
mod types;

pub use impls::BridgeRewardPotAccountDeterminer;

#[frame_support::pallet]
pub mod pallet {
    use sp_std::marker::PhantomData;

    use frame_support::traits::{Hooks, IsType, Get};
    use frame_system::pallet_prelude::BlockNumberFor;

    use xp_mining_common::RewardPotAccountFor;
    use xp_mining_staking::MiningPower;
    use xpallet_assets::BalanceOf;
    use chainx_primitives::AssetId;

    use crate::types::BridgeSubPot;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Target asset id that the bridge serves.
        type TargetAssetId: Get<AssetId>;
        /// Fixed mining power for target asset.
        type TargetAssetMiningPower: Get<MiningPower>;
        /// Reward pot account getter.
        type DetermineRewardPotAccount: RewardPotAccountFor<Self::AccountId, (AssetId, BridgeSubPot)>;
    }


    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
                
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance", BlockNumberFor<T> = "BlockNumber")]
    pub enum Event<T: Config> 
    {
        /// An asset miner claimed the mining reward. [claimer, asset_id, amount]
        Claimed(T::AccountId, AssetId, BalanceOf<T>),
        /// Issue new balance to the reward pot. [reward_pot_account, amount]
        Minted(T::AccountId, BalanceOf<T>, T::AccountId, BalanceOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
       PlaceHolder 
    }
}

