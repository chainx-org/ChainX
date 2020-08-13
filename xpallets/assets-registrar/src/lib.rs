//! This crate provides the feature of managing the foreign assets' meta information.
//!
//! The foreign asset hereby means it's not the native token of the system(PCX for ChainX)
//! but derived from the other blockchain system, e.g., Bitcoin.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

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

macro_rules! define_enum {
    (
    $(#[$attr:meta])*
    $Name:ident { $($Variant:ident),* $(,)* }) =>
    {
        $(#[$attr])*
        pub enum $Name {
            $($Variant),*,
        }
        impl $Name {
            pub fn iter() -> Iter<'static, $Name> {
                static ENUM_ITEMS: &[$Name] = &[$($Name::$Variant),*];
                ENUM_ITEMS.iter()
            }
        }
    }
}

define_enum!(
    #[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Encode, Decode, RuntimeDebug)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    Chain {
        ChainX,
        Bitcoin,
        Ethereum,
        Polkadot,
    }
);

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

pub trait OnAssetRegisterOrRevoke {
    fn on_register(_: &AssetId, _: bool) -> DispatchResult {
        Ok(())
    }
    fn on_revoke(_: &AssetId) -> DispatchResult {
        Ok(())
    }
}

impl OnAssetRegisterOrRevoke for () {}

impl<A: OnAssetRegisterOrRevoke, B: OnAssetRegisterOrRevoke> OnAssetRegisterOrRevoke for (A, B) {
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

    fn on_revoke(id: &AssetId) -> DispatchResult {
        let r = A::on_revoke(id);
        let r2 = B::on_revoke(id);
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
    type OnAssetRegisterOrRevoke: OnAssetRegisterOrRevoke;
}

decl_event!(
    pub enum Event<T> where
    <T as system::Trait>::AccountId
    {
        Register(AssetId, bool),
        Revoke(AssetId),
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
        NotExistedAsset,
        ///
        InvalidAsset,
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

            Self::add_asset(asset_id, asset)?;

            T::OnAssetRegisterOrRevoke::on_register(&asset_id, has_mining_rights)?;
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
            ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);

            Self::remove_asset(&id);
            T::OnAssetRegisterOrRevoke::on_revoke(&id)?;

            Self::deposit_event(RawEvent::Revoke(id));

            Ok(())
        }

        /// recover an offline asset,
        #[weight = 0]
        pub fn recover_asset(origin, #[compact] id: AssetId, has_mining_rights: bool) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::asset_info_of(id).is_some(), Error::<T>::NotExistedAsset);
            ensure!(Self::asset_online(id).is_none(), Error::<T>::InvalidAsset);

            Self::re_add_asset(&id);

            T::OnAssetRegisterOrRevoke::on_register(&id, has_mining_rights)?;
            Self::deposit_event(RawEvent::Register(id, has_mining_rights));
            Ok(())
        }

        #[weight = 0]
        pub fn modify_asset_info(origin, #[compact] id: AssetId, token: Option<Token>, token_name: Option<Token>, desc: Option<Desc>) -> DispatchResult {
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
    /// add an asset into the storage, notice the asset must be valid
    fn add_asset(id: AssetId, asset: AssetInfo) -> DispatchResult {
        let chain = asset.chain();
        if Self::asset_info_of(&id).is_some() {
            Err(Error::<T>::AlreadyExistentToken)?;
        }

        AssetInfoOf::insert(&id, asset);
        AssetOnline::insert(&id, ());

        AssetRegisteredBlock::<T>::insert(&id, system::Module::<T>::block_number());

        AssetIdsOf::mutate(chain, |v| {
            if !v.contains(&id) {
                v.push(id.clone());
            }
        });
        Ok(())
    }

    fn remove_asset(id: &AssetId) {
        AssetOnline::remove(id);
    }

    fn re_add_asset(id: &AssetId) {
        AssetOnline::insert(id, ());
    }

    pub fn asset_ids() -> Vec<AssetId> {
        let mut v = Vec::new();
        for i in Chain::iter() {
            v.extend(Self::asset_ids_of(i));
        }
        v
    }

    pub fn valid_asset_ids() -> Vec<AssetId> {
        Self::asset_ids()
            .into_iter()
            .filter(|id| Self::asset_online(id).is_some())
            .collect()
    }

    pub fn asset_infos() -> BTreeMap<AssetId, AssetInfo> {
        AssetInfoOf::iter().collect()
    }

    pub fn valid_asset_infos() -> BTreeMap<AssetId, AssetInfo> {
        Self::asset_infos()
            .into_iter()
            .filter(|(id, _)| Self::asset_online(id).is_some())
            .collect()
    }

    pub fn get_asset(id: &AssetId) -> result::Result<AssetInfo, DispatchError> {
        if let Some(asset) = Self::asset_info_of(id) {
            if Self::asset_online(id).is_some() {
                Ok(asset)
            } else {
                Err(Error::<T>::InvalidAsset)?
            }
        } else {
            Err(Error::<T>::NotExistedAsset)?
        }
    }

    pub fn ensure_existed_assert(id: &AssetId) -> DispatchResult {
        ensure!(Self::asset_info_of(id).is_some(), Error::<T>::InvalidAsset);
        Ok(())
    }

    pub fn ensure_valid_asset(id: &AssetId) -> DispatchResult {
        ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);
        Ok(())
    }
}

#[inline]
/// Visible ASCII char [0x20, 0x7E]
fn is_ascii_invisible(c: &u8) -> bool {
    *c < 0x20 || *c > 0x7E
}

/// A valid token name should have a legal length and be visible ASCII chars only.
pub fn is_valid_token_name<T: Trait>(name: &[u8]) -> DispatchResult {
    if name.len() > MAX_TOKEN_LEN || name.is_empty() {
        Err(Error::<T>::InvalidAssetNameLen)?;
    }
    xp_runtime::xss_check(name)?;
    for c in name.iter() {
        if is_ascii_invisible(c) {
            Err(Error::<T>::InvalidAsscii)?;
        }
    }
    Ok(())
}

/// A valid desc should be visible ASCII chars only and not too long.
pub fn is_valid_desc<T: Trait>(desc: &[u8]) -> DispatchResult {
    if desc.len() > MAX_DESC_LEN {
        Err(Error::<T>::InvalidDescLen)?;
    }
    xp_runtime::xss_check(desc)?;
    for c in desc.iter() {
        if is_ascii_invisible(c) {
            Err(Error::<T>::InvalidAsscii)?;
        }
    }
    Ok(())
}
/// Token can only use ASCII alphanumeric character or "-.|~".
pub fn is_valid_token<T: Trait>(v: &[u8]) -> DispatchResult {
    if v.len() > MAX_TOKEN_LEN || v.is_empty() {
        Err(Error::<T>::InvalidAssetLen)?;
    }
    let is_valid = |c: &u8| -> bool { c.is_ascii_alphanumeric() || "-.|~".as_bytes().contains(c) };
    for c in v.iter() {
        if !is_valid(c) {
            Err(Error::<T>::InvalidChar)?;
        }
    }
    Ok(())
}
const MAX_TOKEN_LEN: usize = 32;
const MAX_DESC_LEN: usize = 128;
