// Copyright 2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::log::info;

#[cfg(feature = "std")]
use xp_genesis_builder::AllParams;
#[cfg(feature = "std")]
use xpallet_assets::BalanceOf as AssetBalanceOf;
#[cfg(feature = "std")]
use xpallet_mining_staking::BalanceOf as StakingBalanceOf;

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
        pub root_endowed: T::Balance,
        pub initial_authorities_endowed: T::Balance,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                params: Default::default(),
                root_endowed: Default::default(),
                initial_authorities_endowed: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    #[cfg(feature = "std")]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            use crate::genesis::{balances, xassets, xmining_asset, xstaking};

            let now = std::time::Instant::now();

            balances::initialize::<T>(
                &self.params.balances,
                self.root_endowed,
                self.initial_authorities_endowed,
            );
            xassets::initialize::<T>(&self.params.xassets);
            xstaking::initialize::<T>(&self.params.xstaking);
            xmining_asset::initialize::<T>(&self.params.xmining_asset);

            info!(
                target: "runtime::genesis-builder",
                "Took {:?}ms to orchestrate the exported state from ChainX 1.0",
                now.elapsed().as_millis()
            );
        }
    }
}

#[cfg(feature = "std")]
mod genesis {
    pub mod balances {
        use crate::Config;
        use frame_support::traits::StoredMap;
        use pallet_balances::AccountData;
        use xp_genesis_builder::{BalancesParams, FreeBalanceInfo, WellknownAccounts};
        use xp_protocol::X_BTC;
        use xpallet_support::traits::TreasuryAccount;

        /// Returns the validator account by the given reward pot account.
        fn validator_for<'a, T: Config, I: Iterator<Item = &'a (T::AccountId, T::AccountId)>>(
            target_pot: &T::AccountId,
            mut pots: I,
        ) -> Option<&'a T::AccountId> {
            pots.find(|(pot, _)| *pot == *target_pot)
                .map(|(_, validator)| validator)
        }

        pub fn initialize<T: Config>(
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
                legacy_team,
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
                .expect("Set balance can not fail; qed")
            };

            let treasury_account =
                <T as xpallet_mining_staking::Config>::TreasuryAccount::treasury_account();

            let vesting_account = xpallet_mining_staking::Pallet::<T>::vesting_account();

            let mut total_issuance = T::Balance::default();

            for FreeBalanceInfo { who, free } in free_balances {
                if *who == *legacy_council {
                    let treasury_free = *free - root_endowed;
                    set_free_balance(&treasury_account, &treasury_free);
                } else if *who == *legacy_team {
                    let vesting_free = *free - initial_authorities_endowed;
                    set_free_balance(&vesting_account, &vesting_free);
                } else if *who == *legacy_xbtc_pot {
                    let new_xbtc_pot = xpallet_mining_asset::Pallet::<T>::reward_pot_for(&X_BTC);
                    set_free_balance(&new_xbtc_pot, free);
                } else if let Some(validator) = validator_for::<T, _>(who, legacy_pots.iter()) {
                    let new_pot = xpallet_mining_staking::Pallet::<T>::reward_pot_for(validator);
                    set_free_balance(&new_pot, free);
                } else {
                    set_free_balance(who, free);
                }
                total_issuance += *free;
            }

            pallet_balances::TotalIssuance::<T>::mutate(|v| *v = total_issuance);
        }
    }

    pub mod xassets {
        use crate::{AssetBalanceOf, Config};
        use xp_genesis_builder::FreeBalanceInfo;
        use xp_protocol::X_BTC;

        pub fn initialize<T: Config>(
            xbtc_assets: &[FreeBalanceInfo<T::AccountId, AssetBalanceOf<T>>],
        ) {
            for FreeBalanceInfo { who, free } in xbtc_assets {
                xpallet_assets::Pallet::<T>::force_set_free_balance(&X_BTC, who, *free);
            }
        }
    }

    pub mod xstaking {
        use crate::{Config, StakingBalanceOf};
        use xp_genesis_builder::{Nomination, NominatorInfo, XStakingParams};

        pub fn initialize<T: Config>(params: &XStakingParams<T::AccountId, StakingBalanceOf<T>>) {
            let XStakingParams {
                validators,
                nominators,
            } = params;

            let genesis_validators = validators.iter().map(|v| v.who.clone()).collect::<Vec<_>>();

            // Firstly register the genesis validators.
            xpallet_mining_staking::Pallet::<T>::initialize_legacy_validators(validators)
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
                        xpallet_mining_staking::Pallet::<T>::force_set_nominator_vote_weight(
                            nominator, nominee, *weight,
                        );
                        // Skip the validator self-bonding as it has already been processed
                        // in initialize_legacy_validators()
                        if *nominee == *nominator {
                            continue;
                        }
                        xpallet_mining_staking::Pallet::<T>::force_bond(
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
        use crate::Config;
        use xp_genesis_builder::{XBtcMiner, XMiningAssetParams};
        use xp_protocol::X_BTC;

        /// Mining asset module initialization only involves the mining weight.
        /// - Set xbtc mining asset weight.
        /// - Set xbtc miners' weight.
        pub fn initialize<T: Config>(params: &XMiningAssetParams<T::AccountId>) {
            let XMiningAssetParams {
                xbtc_miners,
                xbtc_info,
            } = params;
            let current_block = frame_system::Pallet::<T>::block_number();
            for XBtcMiner { who, weight } in xbtc_miners {
                xpallet_mining_asset::Pallet::<T>::force_set_miner_mining_weight(
                    who,
                    &X_BTC,
                    *weight,
                    current_block,
                );
            }
            xpallet_mining_asset::Pallet::<T>::force_set_asset_mining_weight(
                &X_BTC,
                xbtc_info.weight,
                current_block,
            );
        }
    }
}
