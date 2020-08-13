//! This crate provides the feature of managing the foreign assets' meta information.
//!
//! The foreign asset hereby means it's not the native token of the system(PCX for ChainX)
//! but derived from the other blockchain system, e.g., Bitcoin.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;
mod verifier;

use sp_std::{collections::btree_map::BTreeMap, prelude::*, result, slice::Iter};

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
use frame_system::{self as system, ensure_root};

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
    /// Returns an iterator of all kinds of `Chain`.
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
    fn on_register(id: &AssetId, is_psedu_intention: bool) -> DispatchResult {
        let r = A::on_register(id, is_psedu_intention);
        let r2 = B::on_register(id, is_psedu_intention);
        if r.is_ok() == false {
            return r;
        } else if r2.is_ok() == false {
            return r2;
        }
        Ok(())
    }

    fn on_deregister(id: &AssetId) -> DispatchResult {
        let r = A::on_deregister(id);
        let r2 = B::on_deregister(id);
        if r.is_ok() == false {
            return r;
        } else if r2.is_ok() == false {
            return r2;
        }
        Ok(())
    }
}

pub trait Trait: system::Trait {
    /// Event
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Get Native Id
    type NativeAssetId: Get<AssetId>;

    ///
    type RegistrarHandler: RegistrarHandler;
}

decl_event!(
    pub enum Event<T> where
    <T as system::Trait>::AccountId
    {
        Register(AssetId, bool),
        Deregister(AssetId),
        PlaceHolder(AccountId),
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
        ///
        AlreadyExistentToken,
        ///
        InvalidAsset,
        ///
        AssetDoesNotExist,
    }
}
decl_storage! {
    trait Store for Module<T: Trait> as XAssetsMetadata {
        /// Asset id list for Chain, different Chain has different id list
        pub AssetIdsOf get(fn asset_ids_of): map hasher(twox_64_concat) Chain => Vec<AssetId>;

        /// asset info for every asset, key is asset id
        pub AssetInfoOf get(fn asset_info_of): map hasher(twox_64_concat) AssetId => Option<AssetInfo>;

        ///
        pub AssetOnline get(fn asset_online): map hasher(twox_64_concat) AssetId => Option<()>;

        ///
        pub AssetRegisteredBlock get(fn asset_registered_block): map hasher(twox_64_concat) AssetId => T::BlockNumber;
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
            info!("[register_asset]|id:{:}|{:?}|is_online:{:}|has_mining_rights:{:}", asset_id, asset, is_online, has_mining_rights);

            Self::apply_register(asset_id, asset)?;

            T::RegistrarHandler::on_register(&asset_id, has_mining_rights)?;
            Self::deposit_event(RawEvent::Register(asset_id, has_mining_rights));

            if !is_online {
                let _ = Self::deregister(frame_system::RawOrigin::Root.into(), asset_id.into());
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

        /// recover an offline asset,
        #[weight = 0]
        pub fn recover_asset(origin, #[compact] id: AssetId, has_mining_rights: bool) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::asset_info_of(id).is_some(), Error::<T>::AssetDoesNotExist);
            ensure!(Self::asset_online(id).is_none(), Error::<T>::InvalidAsset);

            AssetOnline::insert(id, ());

            T::RegistrarHandler::on_register(&id, has_mining_rights)?;
            Self::deposit_event(RawEvent::Register(id, has_mining_rights));
            Ok(())
        }

        #[weight = 0]
        pub fn modify_asset_info(
            origin,
            #[compact] id: AssetId,
            token: Option<Token>,
            token_name: Option<Token>,
            desc: Option<Desc>
        ) -> DispatchResult {
            ensure_root(origin)?;
            let mut info = Self::asset_info_of(&id).ok_or(Error::<T>::InvalidAsset)?;

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
    /// Returns all the asset ids of all chains so far.
    #[inline]
    pub fn asset_ids() -> Vec<AssetId> {
        Chain::iter().map(Self::asset_ids_of).flatten().collect()
    }

    /// Returns all the valid asset ids of all chains so far.
    pub fn valid_asset_ids() -> Vec<AssetId> {
        // TODO: extract is_valid_asset() which can be used in serveral places.
        Self::asset_ids()
            .into_iter()
            .filter(Self::is_valid_asset)
            .collect()
    }

    pub fn asset_infos() -> BTreeMap<AssetId, AssetInfo> {
        AssetInfoOf::iter().collect()
    }

    pub fn valid_asset_infos() -> BTreeMap<AssetId, AssetInfo> {
        Self::asset_infos()
            .into_iter()
            .filter(|(id, _)| Self::is_valid_asset(id))
            .collect()
    }

    pub fn get_asset(id: &AssetId) -> result::Result<AssetInfo, DispatchError> {
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

    pub fn ensure_assert_exists(id: &AssetId) -> DispatchResult {
        ensure!(Self::asset_exists(id), Error::<T>::AssetDoesNotExist);
        Ok(())
    }

    pub fn ensure_valid_asset(id: &AssetId) -> DispatchResult {
        ensure!(Self::is_valid_asset(id), Error::<T>::InvalidAsset);
        Ok(())
    }

    /// Actually register an asset.
    fn apply_register(id: AssetId, asset: AssetInfo) -> DispatchResult {
        let chain = asset.chain();
        // FIXME: Self::asset_info_of(&id).is_some() => multiple Error?
        if Self::asset_info_of(&id).is_some() {
            Err(Error::<T>::AlreadyExistentToken)?;
        }

        AssetInfoOf::insert(&id, asset);
        AssetOnline::insert(&id, ());

        AssetRegisteredBlock::<T>::insert(&id, system::Module::<T>::block_number());

        AssetIdsOf::mutate(chain, |v| {
            if !v.contains(&id) {
                v.push(id);
            }
        });
        Ok(())
    }
}
