// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of managing the native and foreign assets' meta information.
//!
//! The foreign asset hereby means it's not the native token of the system(PCX for ChainX)
//! but derived from the other blockchain system, e.g., Bitcoin.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
#[cfg(test)]
mod tests;
mod types;
mod verifier;
pub mod weights;

use sp_std::prelude::*;

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    log::info,
};

use chainx_primitives::{AssetId, Desc, Token};

#[cfg(feature = "std")]
use frame_support::traits::GenesisBuild;

pub use self::types::AssetInfo;
pub use self::weights::WeightInfo;
pub use xp_assets_registrar::{Chain, RegistrarHandler};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    /// The module's config trait.
    ///
    /// `frame_system::Trait` should always be included in our implied traits.
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Native asset Id.
        type NativeAssetId: Get<AssetId>;

        /// Handler for doing stuff after the asset is registered/deregistered.
        type RegistrarHandler: RegistrarHandler;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new foreign asset.
        ///
        /// This is a root-only operation.
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: AssetId,
            asset: AssetInfo,
            is_online: bool,
            has_mining_rights: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;

            asset.is_valid::<T>()?;
            ensure!(!Self::exists(&asset_id), Error::<T>::AssetAlreadyExists);

            info!(
                "[register_asset] id:{}, info:{:?}, is_online:{}, has_mining_rights:{}",
                asset_id, asset, is_online, has_mining_rights
            );

            Self::apply_register(asset_id, asset)?;

            Self::deposit_event(Event::Registered(asset_id, has_mining_rights));
            T::RegistrarHandler::on_register(&asset_id, has_mining_rights)?;

            if !is_online {
                let _ = Self::deregister(frame_system::RawOrigin::Root.into(), asset_id);
            }

            Ok(())
        }

        /// Deregister an asset with given `id`.
        ///
        /// This asset will be marked as invalid.
        ///
        /// This is a root-only operation.
        #[pallet::weight(T::WeightInfo::deregister())]
        pub fn deregister(origin: OriginFor<T>, #[pallet::compact] id: AssetId) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(Self::is_valid(&id), Error::<T>::AssetIsInvalid);

            AssetOnline::<T>::remove(id);

            Self::deposit_event(Event::Deregistered(id));
            T::RegistrarHandler::on_deregister(&id)?;
            Ok(())
        }

        /// Recover a deregister asset to the valid state.
        ///
        /// `RegistrarHandler::on_register()` will be triggered again during the recover process.
        ///
        /// This is a root-only operation.
        #[pallet::weight(T::WeightInfo::recover())]
        pub fn recover(
            origin: OriginFor<T>,
            #[pallet::compact] id: AssetId,
            has_mining_rights: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(Self::exists(&id), Error::<T>::AssetDoesNotExist);
            ensure!(!Self::is_valid(&id), Error::<T>::AssetAlreadyValid);

            AssetOnline::<T>::insert(id, true);

            Self::deposit_event(Event::Recovered(id, has_mining_rights));
            T::RegistrarHandler::on_register(&id, has_mining_rights)?;
            Ok(())
        }

        /// Update the asset info, all the new fields are optional.
        ///
        /// This is a root-only operation.
        #[pallet::weight(T::WeightInfo::update_asset_info())]
        pub fn update_asset_info(
            origin: OriginFor<T>,
            #[pallet::compact] id: AssetId,
            token: Option<Token>,
            token_name: Option<Token>,
            desc: Option<Desc>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut info = Self::asset_info_of(&id).ok_or(Error::<T>::AssetDoesNotExist)?;
            if let Some(t) = token {
                info.set_token(t)
            }
            if let Some(name) = token_name {
                info.set_token_name(name);
            }
            if let Some(desc) = desc {
                info.set_desc(desc);
            }
            AssetInfoOf::<T>::insert(id, info);
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    /// Event for the XAssetRegistrar Pallet
    pub enum Event<T: Config> {
        /// A new asset was registered. [asset_id, has_mining_rights]
        Registered(AssetId, bool),
        /// A deregistered asset was recovered. [asset_id, has_mining_rights]
        Recovered(AssetId, bool),
        /// An asset was deregistered. [asset_id]
        Deregistered(AssetId),
    }

    /// Error for the XAssetRegistrar Pallet
    #[pallet::error]
    pub enum Error<T> {
        /// Token symbol length is zero or too long
        InvalidAssetTokenSymbolLength,
        /// Token symbol char is invalid, only allow ASCII alphanumeric character or '-', '.', '|', '~'
        InvalidAssetTokenSymbolChar,
        /// Token name length is zero or too long
        InvalidAssetTokenNameLength,
        /// Desc length is zero or too long
        InvalidAssetDescLength,
        /// Text is invalid ASCII, only allow ASCII visible character [0x20, 0x7E]
        InvalidAscii,
        /// The asset already exists.
        AssetAlreadyExists,
        /// The asset does not exist.
        AssetDoesNotExist,
        /// The asset is already valid (online), no need to recover.
        AssetAlreadyValid,
        /// The asset is invalid (not online).
        AssetIsInvalid,
    }

    /// Asset id list for each Chain.
    #[pallet::storage]
    #[pallet::getter(fn asset_ids_of)]
    pub(super) type AssetIdsOf<T: Config> =
        StorageMap<_, Twox64Concat, Chain, Vec<AssetId>, ValueQuery>;

    /// Asset info of each asset.
    #[pallet::storage]
    #[pallet::getter(fn asset_info_of)]
    pub(super) type AssetInfoOf<T: Config> = StorageMap<_, Twox64Concat, AssetId, AssetInfo>;

    /// The map of asset to the online state.
    #[pallet::storage]
    #[pallet::getter(fn asset_online)]
    pub(super) type AssetOnline<T: Config> = StorageMap<_, Twox64Concat, AssetId, bool, ValueQuery>;

    /// The map of asset to the block number at which the asset was registered.
    #[pallet::storage]
    #[pallet::getter(fn registered_at)]
    pub(super) type RegisteredAt<T: Config> =
        StorageMap<_, Twox64Concat, AssetId, T::BlockNumber, ValueQuery>;

    /// add_extra_genesis
    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub assets: Vec<(AssetId, AssetInfo, bool, bool)>,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {
                assets: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            let extra_genesis_builder: fn(&Self) = |config| {
                for (id, asset, is_online, has_mining_rights) in &config.assets {
                    Pallet::<T>::register(
                        frame_system::RawOrigin::Root.into(),
                        *id,
                        asset.clone(),
                        *is_online,
                        *has_mining_rights,
                    )
                    .expect("asset registeration during the genesis can not fail");
                }
            };
            extra_genesis_builder(self);
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Returns an iterator of all the asset ids of all chains so far.
    #[inline]
    pub fn asset_ids() -> impl Iterator<Item = AssetId> {
        Chain::iter().map(Self::asset_ids_of).flatten()
    }

    /// Returns an iterator of all the valid asset ids of all chains so far.
    #[inline]
    pub fn valid_asset_ids() -> impl Iterator<Item = AssetId> {
        Self::asset_ids().filter(Self::is_valid)
    }

    /// Returns an iterator of tuple (AssetId, AssetInfo) of all assets.
    #[inline]
    pub fn asset_infos() -> impl Iterator<Item = (AssetId, AssetInfo)> {
        AssetInfoOf::<T>::iter()
    }

    /// Returns an iterator of tuple (AssetId, AssetInfo) of all valid assets.
    #[inline]
    pub fn valid_asset_infos() -> impl Iterator<Item = (AssetId, AssetInfo)> {
        Self::asset_infos().filter(|(id, _)| Self::is_valid(id))
    }

    /// Returns the chain of given asset `asset_id`.
    pub fn chain_of(asset_id: &AssetId) -> Result<Chain, DispatchError> {
        Self::asset_info_of(asset_id)
            .map(|info| info.chain())
            .ok_or_else(|| Error::<T>::AssetDoesNotExist.into())
    }

    /// Returns the asset info of given `id`.
    pub fn get_asset_info(id: &AssetId) -> Result<AssetInfo, DispatchError> {
        if let Some(asset) = Self::asset_info_of(id) {
            if Self::is_valid(id) {
                Ok(asset)
            } else {
                Err(Error::<T>::AssetIsInvalid.into())
            }
        } else {
            Err(Error::<T>::AssetDoesNotExist.into())
        }
    }

    /// Returns true if the given `asset_id` is an online asset.
    pub fn is_online(asset_id: &AssetId) -> bool {
        Self::asset_online(asset_id)
    }

    /// Returns true if the asset info record of given `asset_id` exists.
    pub fn exists(asset_id: &AssetId) -> bool {
        Self::asset_info_of(asset_id).is_some()
    }

    /// Returns true if the asset of given `asset_id` is valid (only check if still online currently).
    pub fn is_valid(asset_id: &AssetId) -> bool {
        Self::is_online(asset_id)
    }

    /// Helper function for checking the asset's existence.
    pub fn ensure_asset_exists(id: &AssetId) -> DispatchResult {
        ensure!(Self::exists(id), Error::<T>::AssetDoesNotExist);
        Ok(())
    }

    /// Helper function for checking the asset's validity.
    pub fn ensure_asset_is_valid(id: &AssetId) -> DispatchResult {
        ensure!(Self::is_valid(id), Error::<T>::AssetIsInvalid);
        Ok(())
    }

    /// Actually register an asset.
    fn apply_register(id: AssetId, asset: AssetInfo) -> DispatchResult {
        let chain = asset.chain();
        AssetIdsOf::<T>::mutate(chain, |ids| {
            if !ids.contains(&id) {
                ids.push(id);
            }
        });

        AssetInfoOf::<T>::insert(&id, asset);
        AssetOnline::<T>::insert(&id, true);

        RegisteredAt::<T>::insert(&id, frame_system::Pallet::<T>::block_number());

        Ok(())
    }
}

#[cfg(feature = "std")]
impl GenesisConfig {
    /// Direct implementation of `GenesisBuild::assimilate_storage`.
    ///
    /// Kept in order not to break dependency.
    pub fn assimilate_storage<T: Config>(
        &self,
        storage: &mut sp_runtime::Storage,
    ) -> Result<(), String> {
        <Self as GenesisBuild<T>>::assimilate_storage(self, storage)
    }
}
