//! This crate provides the feature of managing the foreign assets' meta information.
//!
//! The foreign asset hereby means it's not the native token of the system(PCX for ChainX)
//! but derived from the other blockchain system, e.g., Bitcoin.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;
mod verifier;

use sp_std::{prelude::*, result, slice::Iter};

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    IterableStorageMap, RuntimeDebug,
};
use frame_system::ensure_root;

// ChainX
use chainx_primitives::{AssetId, Decimals, Desc, Token};
use xpallet_support::info;

pub use verifier::*;

#[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Chain {
    ChainX,
    Bitcoin,
    Ethereum,
    Polkadot,
}

const CHAINS: [Chain; 4] = [
    Chain::ChainX,
    Chain::Bitcoin,
    Chain::Ethereum,
    Chain::Polkadot,
];

impl Chain {
    /// Returns an iterator of all `Chain`.
    pub fn iter() -> Iter<'static, Chain> {
        CHAINS.iter()
    }
}

impl Default for Chain {
    fn default() -> Self {
        Chain::ChainX
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct AssetInfo {
    token: Token,
    token_name: Token,
    chain: Chain,
    decimals: Decimals,
    desc: Desc,
}

impl sp_std::fmt::Debug for AssetInfo {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(
            f,
            "AssetInfo: {{token: {}, token_name: {}, chain: {:?}, decimals: {}, desc: {}}}",
            String::from_utf8_lossy(&self.token).into_owned(),
            String::from_utf8_lossy(&self.token_name).into_owned(),
            self.chain,
            self.decimals,
            String::from_utf8_lossy(&self.desc).into_owned()
        )
    }
    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl AssetInfo {
    pub fn new<T: Trait>(
        token: Token,
        token_name: Token,
        chain: Chain,
        decimals: Decimals,
        desc: Desc,
    ) -> result::Result<Self, DispatchError> {
        let a = AssetInfo {
            token,
            token_name,
            chain,
            decimals,
            desc,
        };
        a.is_valid::<T>()?;
        Ok(a)
    }

    pub fn is_valid<T: Trait>(&self) -> DispatchResult {
        is_valid_token::<T>(&self.token)?;
        is_valid_token_name::<T>(&self.token_name)?;
        is_valid_desc::<T>(&self.desc)
    }

    pub fn token(&self) -> &Token {
        &self.token
    }

    pub fn token_name(&self) -> &Token {
        &self.token_name
    }

    pub fn chain(&self) -> Chain {
        self.chain
    }

    pub fn desc(&self) -> &Desc {
        &self.desc
    }

    pub fn decimals(&self) -> Decimals {
        self.decimals
    }

    pub fn set_desc(&mut self, desc: Desc) {
        self.desc = desc
    }

    pub fn set_token(&mut self, token: Token) {
        self.token = token
    }

    pub fn set_token_name(&mut self, token_name: Token) {
        self.token_name = token_name
    }
}

/// Trait for doing some stuff on the registration/deregistration of a foreign asset.
pub trait RegistrarHandler {
    fn on_register(_: &AssetId, _: bool) -> DispatchResult {
        Ok(())
    }

    fn on_deregister(_: &AssetId) -> DispatchResult {
        Ok(())
    }
}

impl RegistrarHandler for () {}

impl<A: RegistrarHandler, B: RegistrarHandler> RegistrarHandler for (A, B) {
    fn on_register(id: &AssetId, has_mining_rights: bool) -> DispatchResult {
        A::on_register(id, has_mining_rights)?;
        B::on_register(id, has_mining_rights)?;
        Ok(())
    }

    fn on_deregister(id: &AssetId) -> DispatchResult {
        A::on_deregister(id)?;
        B::on_deregister(id)?;
        Ok(())
    }
}

pub trait Trait: frame_system::Trait {
    /// Event
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Get Native Id
    type NativeAssetId: Get<AssetId>;

    /// Handler for doing stuff after the asset is registered/deregistered.
    type RegistrarHandler: RegistrarHandler;
}

decl_event!(
    pub enum Event<T> where <T as frame_system::Trait>::AccountId {
        /// A new asset is registered. [asset_id, has_mining_rights]
        Register(AssetId, bool),
        /// A deregistered asset is recovered. [asset_id, has_mining_rights]
        Recover(AssetId, bool),
        /// An asset is invalid now. [asset_id]
        Deregister(AssetId),
        PhantomData(AccountId),
    }
);

decl_error! {
    /// Error for the Assets Metadata Module
    pub enum Error for Module<T: Trait> {
        /// Token length is zero or too long
        InvalidAssetLen,
        /// Token name length is zero or too long
        InvalidAssetNameLen,
        /// Desc length is zero or too long
        InvalidDescLen,
        /// Memo length is zero or too long
        InvalidMemoLen,
        /// only allow ASCII alphanumeric character or '-', '.', '|', '~'
        InvalidChar,
        /// only allow ASCII alphanumeric character
        InvalidAsscii,
        /// The asset already exists.
        AssetAlreadyExists,
        /// The asset is already valid, no need to recover.
        AssetAlreadyValid,
        /// The asset is not online.
        InvalidAsset,
        /// The asset does not exist.
        AssetDoesNotExist,
    }
}
decl_storage! {
    trait Store for Module<T: Trait> as XAssetsRegistrar {
        /// Asset id list for each Chain.
        pub AssetIdsOf get(fn asset_ids_of): map hasher(twox_64_concat) Chain => Vec<AssetId>;

        /// Asset info of each asset.
        pub AssetInfoOf get(fn asset_info_of): map hasher(twox_64_concat) AssetId => Option<AssetInfo>;

        /// The map of asset to the online state.
        pub AssetOnline get(fn asset_online): map hasher(twox_64_concat) AssetId => Option<()>;

        /// The map of asset to the block number at which the asset was registered.
        pub RegisteredAt get(fn registered_at): map hasher(twox_64_concat) AssetId => T::BlockNumber;
    }
    add_extra_genesis {
        config(assets): Vec<(AssetId, AssetInfo, bool, bool)>;
        build(|config| {
            Module::<T>::initialize_assets(&config.assets);
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        /// Register a new foreign asset.
        ///
        /// This is a root-only operation.
        #[weight = 0]
        pub fn register(
            origin,
            #[compact] asset_id: AssetId,
            asset: AssetInfo,
            is_online: bool,
            has_mining_rights: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;
            asset.is_valid::<T>()?;
            ensure!(!Self::asset_exists(&asset_id), Error::<T>::AssetAlreadyExists);

            info!("[register_asset]|id:{:}|{:?}|is_online:{:}|has_mining_rights:{:}", asset_id, asset, is_online, has_mining_rights);

            Self::apply_register(asset_id, asset)?;

            T::RegistrarHandler::on_register(&asset_id, has_mining_rights)?;
            Self::deposit_event(RawEvent::Register(asset_id, has_mining_rights));

            if !is_online {
                let _ = Self::deregister(frame_system::RawOrigin::Root.into(), asset_id);
            }

            Ok(())
        }

        /// Deregister an asset with given `id`.
        ///
        /// This asset will be marked as invalid.
        #[weight = 0]
        pub fn deregister(origin, #[compact] id: AssetId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::is_valid_asset(&id), Error::<T>::InvalidAsset);

            AssetOnline::remove(id);
            T::RegistrarHandler::on_deregister(&id)?;

            Self::deposit_event(RawEvent::Deregister(id));

            Ok(())
        }

        /// Recover a deregister asset to the valid state.
        ///
        /// `RegistrarHandler::on_register()` will be triggered again during the recover process.
        #[weight = 0]
        pub fn recover(origin, #[compact] id: AssetId, has_mining_rights: bool) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::asset_exists(&id), Error::<T>::AssetDoesNotExist);
            ensure!(!Self::is_valid_asset(&id), Error::<T>::AssetAlreadyValid);

            AssetOnline::insert(id, ());

            T::RegistrarHandler::on_register(&id, has_mining_rights)?;
            Self::deposit_event(RawEvent::Recover(id, has_mining_rights));
            Ok(())
        }

        /// Update the asset info, all the new fields are optional.
        #[weight = 0]
        pub fn update_asset_info(
            origin,
            #[compact] id: AssetId,
            token: Option<Token>,
            token_name: Option<Token>,
            desc: Option<Desc>
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut info = Self::asset_info_of(&id).ok_or(Error::<T>::AssetDoesNotExist)?;

            token.map(|t| info.set_token(t));
            token_name.map(|name| info.set_token_name(name));
            desc.map(|desc| info.set_desc(desc));

            AssetInfoOf::insert(id, info);
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn initialize_assets(assets: &Vec<(AssetId, AssetInfo, bool, bool)>) {
        for (id, asset, is_online, has_mining_rights) in assets {
            Self::register(
                frame_system::RawOrigin::Root.into(),
                *id,
                asset.clone(),
                *is_online,
                *has_mining_rights,
            )
            .expect("asset registeration during the genesis can not fail");
        }
    }
}

impl<T: Trait> Module<T> {
    /// Returns an iterator of all the asset ids of all chains so far.
    #[inline]
    pub fn asset_ids() -> impl Iterator<Item = AssetId> {
        Chain::iter().map(Self::asset_ids_of).flatten()
    }

    /// Returns an iterator of all the valid asset ids of all chains so far.
    #[inline]
    pub fn valid_asset_ids() -> impl Iterator<Item = AssetId> {
        Self::asset_ids().filter(Self::is_valid_asset)
    }

    /// Returns an iterator of tuple (AssetId, AssetInfo) of all assets.
    #[inline]
    pub fn asset_infos() -> impl Iterator<Item = (AssetId, AssetInfo)> {
        AssetInfoOf::iter()
    }

    /// Returns an iterator of tuple (AssetId, AssetInfo) of all valid assets.
    #[inline]
    pub fn valid_asset_infos() -> impl Iterator<Item = (AssetId, AssetInfo)> {
        Self::asset_infos().filter(|(id, _)| Self::is_valid_asset(id))
    }

    /// Returns the asset info of given `id`.
    pub fn get_asset_info(id: &AssetId) -> result::Result<AssetInfo, DispatchError> {
        if let Some(asset) = Self::asset_info_of(id) {
            if Self::is_valid_asset(id) {
                Ok(asset)
            } else {
                Err(Error::<T>::InvalidAsset)?
            }
        } else {
            Err(Error::<T>::AssetDoesNotExist)?
        }
    }

    /// Returns true if the asset info record of given `asset_id` exists.
    pub fn asset_exists(asset_id: &AssetId) -> bool {
        Self::asset_info_of(&asset_id).is_some()
    }

    /// Returns true if the asset of given `asset_id` is still online.
    pub fn is_valid_asset(asset_id: &AssetId) -> bool {
        Self::asset_online(asset_id).is_some()
    }

    /// Helper function for checking the asset's existence.
    pub fn ensure_assert_exists(id: &AssetId) -> DispatchResult {
        ensure!(Self::asset_exists(id), Error::<T>::AssetDoesNotExist);
        Ok(())
    }

    /// Helper function for checking the asset's validity.
    pub fn ensure_asset_is_valid(id: &AssetId) -> DispatchResult {
        ensure!(Self::is_valid_asset(id), Error::<T>::InvalidAsset);
        Ok(())
    }

    /// Actually register an asset.
    fn apply_register(id: AssetId, asset: AssetInfo) -> DispatchResult {
        let chain = asset.chain();
        AssetIdsOf::mutate(chain, |v| {
            if !v.contains(&id) {
                v.push(id);
            }
        });

        AssetInfoOf::insert(&id, asset);
        AssetOnline::insert(&id, ());

        RegisteredAt::<T>::insert(&id, frame_system::Module::<T>::block_number());

        Ok(())
    }
}
