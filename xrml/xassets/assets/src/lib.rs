// Copyright 2018 Chainpool.
//! Assets: Handles token asset balances.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
extern crate substrate_primitives;

// for substrate runtime
extern crate sr_std as rstd;

extern crate sr_io as runtime_io;
extern crate sr_primitives as primitives;

// for substrate runtime module lib
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;

extern crate xr_primitives;

extern crate xrml_xsupport as xsupport;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod assetdef;
pub mod memo;

use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_support::dispatch::Result;

use primitives::traits::{CheckedAdd, CheckedSub, Zero};
use runtime_support::{StorageMap, StorageValue};

// substrate mod
use system::ensure_signed;

use xsupport::storage::btree_map::CodecBTreeMap;

pub use assetdef::{
    is_valid_desc, is_valid_token, Asset, Chain, ChainT, Desc, DescString, Precision, Token,
    TokenString,
};

pub use memo::{is_valid_memo, Memo};

pub type Address<AccountId, AccountIndex> = balances::address::Address<AccountId, AccountIndex>;

pub trait Trait: balances::Trait {
    /// Event
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type OnAssetChanged: OnAssetChanged<Self::AccountId, Self::Balance>;

    type OnAssetRegistration: OnAssetRegistration;
}

pub trait OnAssetChanged<AccountId, Balance> {
    fn on_move(from: &AccountId, to: &AccountId, token: &Token, value: Balance);
    fn on_issue(who: &AccountId, token: &Token, value: Balance) -> Result;
    fn on_destroy(who: &AccountId, token: &Token, value: Balance);
    fn on_reserve(_who: &AccountId, _token: &Token, _value: Balance) {}
    fn on_unreserve(_who: &AccountId, _token: &Token, _value: Balance) {}
    fn on_set_balance(_who: &AccountId, _token: &Token, _type: AssetType, _value: Balance) {}
}

impl<AccountId, Balance> OnAssetChanged<AccountId, Balance> for () {
    fn on_move(_: &AccountId, _: &AccountId, _: &Token, _: Balance) {}
    fn on_issue(_: &AccountId, _: &Token, _: Balance) -> Result {
        Ok(())
    }
    fn on_destroy(_: &AccountId, _: &Token, _: Balance) {}
}

pub trait OnAssetRegistration {
    fn register_psedu_intention(_: Token) -> Result;
}

impl OnAssetRegistration for () {
    fn register_psedu_intention(_: Token) -> Result {
        Ok(())
    }
}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum AssetType {
    Free,
    ReservedStaking,
    ReservedWithdrawal,
    ReservedDexSpot,
    ReservedDexFuture,
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Free
    }
}

decl_event!(
    pub enum Event<T> where
        <T as balances::Trait>::Balance
    {
        Fee(Balance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// register_asset to module, should allow by root
        fn register_asset(asset: Asset, is_psedu_intention: bool, free: T::Balance) -> Result {
            asset.is_valid()?;
            if is_psedu_intention {
                T::OnAssetRegistration::register_psedu_intention(asset.token())?;
            }
            Self::add_asset(asset, free)?;
            Ok(())
        }

        /// cancel asset, mark this asset is invalid
        fn cancel_asset(token: Token) -> Result {
            is_valid_token(&token)?;
            Self::remove_asset(&token)?;
            Ok(())
        }

        /// set free token for an account
        fn set_balance(who: Address<T::AccountId, T::AccountIndex>, token: Token, balances: CodecBTreeMap<AssetType, T::Balance>) -> Result {
            let who = balances::Module::<T>::lookup(who)?;
            Self::set_balance_by_root(&who, &token, balances)?;
            Ok(())
        }

        /// transfer between account
        fn transfer(origin, dest: Address<T::AccountId, T::AccountIndex>, token: Token, value: T::Balance, memo: Memo) -> Result {
            runtime_io::print("[xassets] transfer");
            let transactor = ensure_signed(origin)?;
            let dest = balances::Module::<T>::lookup(dest)?;

            is_valid_memo::<T>(&memo)?;

            Self::move_free_balance(&transactor, &dest, &token, value).map_err(|e| e.info())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XAssets {
        /// Asset token index list for Chain, different Chain has different token list
        pub AssetList get(asset_list): map Chain => Vec<Token>;

        /// asset info for every token, key is token token
        pub AssetInfo get(asset_info): map Token => Option<(Asset, bool, T::BlockNumber)>;

        /// asset list of a account
        pub CrossChainAssetsOf get(crosschain_assets_of): map T::AccountId => Vec<Token>;

        /// asset balance for user&token, use btree_map to accept different asset type
        pub AssetBalance: map (T::AccountId, Token) => CodecBTreeMap<AssetType, T::Balance>;
        /// asset balance for a token, use btree_map to accept different asset type
        pub TotalAssetBalance: map Token => CodecBTreeMap<AssetType, T::Balance>;

        /// price
        pub PCXPriceFor get(pcx_price_for): map Token => Option<T::Balance>;

        /// memo len
        pub MemoLen get(memo_len) config(): u32;
    }
    add_extra_genesis {
        config(asset_list): Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
        config(pcx): (Precision, Desc);
        build(|storage: &mut primitives::StorageMap, _: &mut primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
                use runtime_io::with_externalities;
                use substrate_primitives::Blake2Hasher;
                use primitives::traits::{Zero, As};
                let src_r = storage.clone().build_storage().unwrap().0;
                let mut tmp_storage: runtime_io::TestExternalities<Blake2Hasher> = src_r.into();
                with_externalities(&mut tmp_storage, || {
                    let chainx: Token = <Module<T> as ChainT>::TOKEN.to_vec();
                    let pcx = Asset::new(chainx, Chain::ChainX, config.pcx.0, config.pcx.1.clone()).unwrap();
                    Module::<T>::register_asset(pcx, false, Zero::zero()).unwrap();
                    // init for asset_list
                    for (asset, is_psedu_intention, init_list) in config.asset_list.iter() {
                        let t = asset.token();
                        Module::<T>::register_asset(asset.clone(), *is_psedu_intention, Zero::zero()).unwrap();

                        for (accountid, value) in init_list {
                            Module::<T>::issue(&accountid, &t, As::sa(*value)).unwrap();
                        }
                    }

                });
                let map: primitives::StorageMap = tmp_storage.into();
                storage.extend(map);
        });
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"PCX";
    fn chain() -> Chain {
        Chain::ChainX
    }
}

impl<T: Trait> Module<T> {
    // token storage
    pub fn asset_balance(who: &T::AccountId, token: &Token, type_: AssetType) -> T::Balance {
        if token.as_slice() == <Self as ChainT>::TOKEN && type_ == AssetType::Free {
            balances::Module::<T>::free_balance(who)
        } else {
            *AssetBalance::<T>::get(&(who.clone(), token.clone()))
                .0
                .get(&type_)
                .unwrap_or(&Zero::zero())
        }
    }

    fn set_asset_balance(who: &T::AccountId, token: &Token, type_: AssetType, val: T::Balance) {
        if token.as_slice() == <Self as ChainT>::TOKEN && type_ == AssetType::Free {
            balances::Module::<T>::set_free_balance(who, val);
        } else {
            AssetBalance::<T>::mutate(&(who.clone(), token.clone()), |m| {
                let _ = m.0.insert(type_, val); // update the value
            });
        }
    }

    /// free balance for a account for a token
    pub fn free_balance(who: &T::AccountId, token: &Token) -> T::Balance {
        Self::asset_balance(who, token, AssetType::Free)
    }

    fn set_free_balance(who: &T::AccountId, token: &Token, value: T::Balance) {
        Self::set_asset_balance(who, token, AssetType::Free, value)
    }

    fn set_free_balance_creating(who: &T::AccountId, token: &Token, value: T::Balance) {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            balances::Module::<T>::set_free_balance_creating(who, value);
        } else {
            let need_create = balances::FreeBalance::<T>::exists(who);
            if need_create == false {
                balances::Module::<T>::set_free_balance_creating(who, Zero::zero());
            }
            Self::set_free_balance(who, token, value)
        }
    }

    pub fn total_asset_balance(token: &Token, type_: AssetType) -> T::Balance {
        if token.as_slice() == <Self as ChainT>::TOKEN && type_ == AssetType::Free {
            let other_types = TotalAssetBalance::<T>::get(token)
                .0
                .iter()
                .filter(|(&k, _)| k != AssetType::Free) // remove free calc
                .fold(Zero::zero(), |acc, (_, v)| acc + *v);
            balances::TotalIssuance::<T>::get() - other_types
        } else {
            *TotalAssetBalance::<T>::get(token)
                .0
                .get(&type_)
                .unwrap_or(&Zero::zero())
        }
    }

    fn set_total_asset_balance(token: &Token, type_: AssetType, value: T::Balance) {
        if token.as_slice() == <Self as ChainT>::TOKEN && type_ == AssetType::Free {
            // do nothing
        } else {
            TotalAssetBalance::<T>::mutate(token, |m| {
                let _ = m.0.insert(type_, value); // update the value
            });
        }
    }

    /// all type balance of `who` for token
    pub fn all_type_balance_of(who: &T::AccountId, token: &Token) -> T::Balance {
        let key = (who.clone(), token.clone());
        if token.as_slice() == <Self as ChainT>::TOKEN {
            let mut b: T::Balance = Zero::zero();
            b += balances::FreeBalance::<T>::get(who);
            b += AssetBalance::<T>::get(&key)
                .0
                .iter()
                .filter(|(&k, _)| k != AssetType::Free) // remove free calc
                .fold(Zero::zero(), |acc, (_, v)| acc + *v);
            b
        } else {
            AssetBalance::<T>::get(&key)
                .0
                .iter()
                .fold(Zero::zero(), |acc, (_, v)| acc + *v)
        }
    }

    /// all type balance of a token
    pub fn all_type_balance(token: &Token) -> T::Balance {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            balances::TotalIssuance::<T>::get()
        } else {
            TotalAssetBalance::<T>::get(token)
                .0
                .iter()
                .fold(Zero::zero(), |acc, (_, v)| acc + *v)
        }
    }

    pub fn should_not_free_type(type_: AssetType) -> Result {
        if type_ == AssetType::Free {
            return Err("should not be free type here");
        }
        Ok(())
    }

    pub fn should_not_chainx(token: &Token) -> Result {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            return Err("should not use chainx token here");
        }
        Ok(())
    }
}

// asset related
impl<T: Trait> Module<T> {
    /// add an asset into the storage, notice the asset must be valid
    fn add_asset(asset: Asset, free: T::Balance) -> Result {
        let token = asset.token();
        let chain = asset.chain();
        if AssetInfo::<T>::exists(&token) {
            return Err("already has this token");
        }

        AssetInfo::<T>::insert(&token, (asset, true, system::Module::<T>::block_number()));

        AssetList::<T>::mutate(chain, |v| {
            v.push(token.clone());
        });

        Self::init_asset_balance(&token, free);
        Ok(())
    }

    fn init_asset_balance(token: &Token, free: T::Balance) {
        Self::set_total_asset_balance(token, AssetType::Free, free);
    }

    fn remove_asset(token: &Token) -> Result {
        if let Some(mut info) = AssetInfo::<T>::get(token) {
            let chain = info.0.chain();
            info.1 = false;
            AssetInfo::<T>::insert(token.clone(), info);
            // remove this token index from AssetList
            AssetList::<T>::mutate(chain, |v| {
                v.retain(|i| i != token);
            });

            Ok(())
        } else {
            Err("this token dose not register yet or is invalid")
        }
    }

    pub fn is_valid_asset(token: &Token) -> Result {
        is_valid_token(token)?;
        if let Some(info) = Self::asset_info(token) {
            if info.1 == true {
                return Ok(());
            }
            return Err("not a valid token");
        }
        Err("not a registered token")
    }

    pub fn is_valid_asset_for(who: &T::AccountId, token: &Token) -> Result {
        Self::is_valid_asset(token)?;
        // if it's native asset
        if let Some((asset, _, _)) = Self::asset_info(token) {
            if let Chain::ChainX = asset.chain() {
                return Ok(());
            }
        }

        if Self::crosschain_assets_of(who).contains(token) {
            Ok(())
        } else {
            Err("not a existed token in this account token list")
        }
    }

    pub fn assets() -> Vec<Token> {
        let mut v = Vec::new();
        for i in Chain::iterator() {
            v.extend(Self::asset_list(i));
        }
        v
    }

    pub fn assets_of(who: &T::AccountId) -> Vec<Token> {
        let mut v = Self::asset_list(Chain::default()); // default is ChainX
        v.extend(Self::crosschain_assets_of(who));
        v
    }

    pub fn native_assets() -> Vec<Token> {
        Self::asset_list(Chain::ChainX)
    }

    pub fn crosschain_assets() -> Vec<Token> {
        let mut v: Vec<Token> = Vec::new();
        for c in Chain::iterator() {
            if *c != Chain::default() {
                // all assets except ChainX
                v.extend(Self::asset_list(c));
            }
        }
        v
    }

    /// notice don't call this func in runtime
    pub fn valid_assets() -> Vec<Token> {
        Self::assets()
            .into_iter()
            .filter(|t| {
                if let Some(t) = Self::asset_info(t) {
                    t.1
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn get_asset(token: &Token) -> StdResult<Asset, &'static str> {
        if let Some((asset, valid, _)) = Self::asset_info(token) {
            if valid == false {
                return Err("this asset is invalid, maybe has cancelled.");
            }
            Ok(asset)
        } else {
            return Err("this token asset not exist!");
        }
    }
}

/// token issue destroy reserve/unreserve
impl<T: Trait> Module<T> {
    fn init_asset_for(who: &T::AccountId, token: &Token) {
        if let Err(_) = Self::is_valid_asset_for(who, token) {
            <CrossChainAssetsOf<T>>::mutate(who, |assets| assets.push(token.clone()));
        }
    }

    pub fn issue(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        Self::should_not_chainx(token)?;
        Self::is_valid_asset(token)?;

        let total_free_token = Self::total_asset_balance(token, AssetType::Free);
        let free_token = Self::asset_balance(who, token, AssetType::Free);
        // check
        let new_free_token = match free_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("free balance too high to issue"),
        };
        let new_total_free_token = match total_free_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("total free balance too high to issue"),
        };
        // set to storage
        Self::init_asset_for(who, token);

        Self::set_total_asset_balance(token, AssetType::Free, new_total_free_token);
        Self::set_asset_balance(who, token, AssetType::Free, new_free_token);

        T::OnAssetChanged::on_issue(who, token, value)?;
        Ok(())
    }

    pub fn reserve(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
        type_: AssetType,
    ) -> Result {
        Self::is_valid_asset_for(who, token)?;
        Self::should_not_free_type(type_)?;

        // get from storage
        let total_free_token = Self::total_asset_balance(token, AssetType::Free);
        let total_reserved_token = Self::total_asset_balance(token, type_);
        let free_token = Self::asset_balance(who, token, AssetType::Free);
        let reserved_token = Self::asset_balance(who, token, type_);
        // test overflow
        let new_free_token = match free_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("free balance too low to reserve"),
        };
        let new_reserved_token = match reserved_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("reserved balance too high to reserve"),
        };
        let new_total_free_token = match total_free_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("total free balance too low to reserve"),
        };
        let new_total_reserved_token = match total_reserved_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("total reserved balance too high to reserve"),
        };
        // set to storage
        Self::set_total_asset_balance(token, AssetType::Free, new_total_free_token);
        Self::set_total_asset_balance(token, type_, new_total_reserved_token);
        Self::set_asset_balance(who, token, AssetType::Free, new_free_token);
        Self::set_asset_balance(who, token, type_, new_reserved_token);

        T::OnAssetChanged::on_reserve(who, token, value);
        Ok(())
    }

    pub fn unreserve(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
        type_: AssetType,
    ) -> Result {
        Self::is_valid_asset_for(who, token)?;
        Self::should_not_free_type(type_)?;

        // get from storage
        let total_free_token = Self::total_asset_balance(token, AssetType::Free);
        let total_reserved_token = Self::total_asset_balance(token, type_);
        let free_token = Self::asset_balance(who, token, AssetType::Free);
        let reserved_token = Self::asset_balance(who, token, type_);

        // test overflow
        let new_free_token = match free_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("free balance too high to unreserve"),
        };
        let new_reserved_token = match reserved_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("reserved balance too low to unreserve"),
        };
        let new_total_free_token = match total_free_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("total free balance too high to unreserve"),
        };
        let new_total_reserved_token = match total_reserved_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("total reserved balance too low to unreserve"),
        };
        // set to storage
        Self::set_total_asset_balance(token, AssetType::Free, new_total_free_token);
        Self::set_total_asset_balance(token, type_, new_total_reserved_token);
        Self::set_asset_balance(who, token, AssetType::Free, new_free_token);
        Self::set_asset_balance(who, token, type_, new_reserved_token);

        T::OnAssetChanged::on_unreserve(who, token, value);
        Ok(())
    }

    pub fn destroy(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
        type_: AssetType,
    ) -> Result {
        Self::should_not_chainx(token)?;
        Self::is_valid_asset_for(who, token)?;
        Self::should_not_free_type(type_)?;

        // get storage
        let total_reserved_token = Self::total_asset_balance(token, type_);
        let reserved_token = Self::asset_balance(who, token, type_);
        // check
        let new_reserved_token = match reserved_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("reserved balance too low to destroy"),
        };
        let new_total_reserved_token = match total_reserved_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("total reserved balance too low to destroy"),
        };

        // set to storage
        Self::set_total_asset_balance(token, type_, new_total_reserved_token);
        Self::set_asset_balance(who, token, type_, new_reserved_token);

        T::OnAssetChanged::on_destroy(who, token, value);
        Ok(())
    }

    pub fn move_free_balance(
        from: &T::AccountId,
        to: &T::AccountId,
        token: &Token,
        value: T::Balance,
    ) -> StdResult<(), TokenErr> {
        Self::is_valid_asset_for(from, token).map_err(|_| TokenErr::InvalidToken)?;

        if from == to {
            return Err(TokenErr::SomeAccount);
        }

        if token.as_slice() != <Self as ChainT>::TOKEN {
            Self::init_asset_for(to, token);
        }

        let from_token: T::Balance = Self::free_balance(from, token);
        let to_token: T::Balance = Self::free_balance(to, token);

        let new_from_token = match from_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err(TokenErr::NotEnough),
        };
        let new_to_token = match to_token.checked_add(&value) {
            Some(b) => b,
            None => return Err(TokenErr::OverFlow),
        };

        Self::set_free_balance(from, token, new_from_token);
        Self::set_free_balance_creating(to, token, new_to_token);
        T::OnAssetChanged::on_move(from, to, token, value);
        Ok(())
    }

    pub fn set_balance_by_root(
        who: &T::AccountId,
        token: &Token,
        balances: CodecBTreeMap<AssetType, T::Balance>,
    ) -> Result {
        for (type_, val) in balances.0.into_iter() {
            let old_val = Self::asset_balance(who, token, type_);
            let old_total_val = Self::total_asset_balance(token, type_);
            if old_val == val {
                continue;
            }

            let new_total_val = if val > old_val {
                match val.checked_sub(&old_val) {
                    None => return Err("balance too low to sub value"),
                    Some(b) => match old_total_val.checked_add(&b) {
                        None => return Err("old total balance too high to add value"),
                        Some(new) => new,
                    },
                }
            } else {
                match old_val.checked_sub(&val) {
                    None => return Err("old balance too low to sub value"),
                    Some(b) => match old_total_val.checked_sub(&b) {
                        None => return Err("old total balance too low to sub value"),
                        Some(new) => new,
                    },
                }
            };

            Self::set_asset_balance(who, token, type_, val);
            if token.as_slice() == <Self as ChainT>::TOKEN && type_ == AssetType::Free {
                balances::TotalIssuance::<T>::put(new_total_val)
            } else {
                Self::set_total_asset_balance(token, type_, new_total_val);
            }
            T::OnAssetChanged::on_set_balance(who, token, type_, val);
        }
        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum TokenErr {
    NotEnough,
    OverFlow,
    SomeAccount,
    InvalidToken,
    InvalidAccount,
}

impl TokenErr {
    pub fn info(&self) -> &'static str {
        match *self {
            TokenErr::NotEnough => "free balance too low",
            TokenErr::OverFlow => "overflow for this value",
            TokenErr::SomeAccount => "from and to are same account",
            TokenErr::InvalidToken => "not a valid token for this account",
            TokenErr::InvalidAccount => "Account Locked",
        }
    }
}

// wrapper for balances module
impl<T: Trait> Module<T> {
    pub fn pcx_free_balance(who: &T::AccountId) -> T::Balance {
        Self::free_balance(who, &<Self as ChainT>::TOKEN.to_vec())
    }

    pub fn pcx_total_balance(who: &T::AccountId) -> T::Balance {
        Self::all_type_balance_of(who, &<Self as ChainT>::TOKEN.to_vec())
    }

    pub fn pcx_set_free_balance(who: &T::AccountId, value: T::Balance) {
        Self::set_free_balance(who, &<Self as ChainT>::TOKEN.to_vec(), value);
    }

    pub fn pcx_reward(who: &T::AccountId, value: T::Balance) -> Result {
        balances::Module::<T>::reward(who, value)
    }

    pub fn pcx_staking_reserve(who: &T::AccountId, value: T::Balance) -> Result {
        Self::reserve(
            who,
            &<Self as ChainT>::TOKEN.to_vec(),
            value,
            AssetType::ReservedStaking,
        )
    }

    pub fn pcx_staking_unreserve(who: &T::AccountId, value: T::Balance) -> Result {
        Self::unreserve(
            who,
            &<Self as ChainT>::TOKEN.to_vec(),
            value,
            AssetType::ReservedStaking,
        )
    }

    pub fn increase_total_stake_by(value: T::Balance) {
        balances::Module::<T>::increase_total_stake_by(value);
    }

    pub fn lookup_index(index: T::AccountIndex) -> Option<T::AccountId> {
        balances::Module::<T>::lookup_index(index)
    }

    pub fn lookup_address(a: Address<T::AccountId, T::AccountIndex>) -> Option<T::AccountId> {
        balances::Module::<T>::lookup_address(a)
    }

    pub fn lookup(
        a: Address<T::AccountId, T::AccountIndex>,
    ) -> StdResult<T::AccountId, &'static str> {
        match a {
            balances::address::Address::Id(i) => Ok(i),
            balances::address::Address::Index(i) => {
                balances::Module::<T>::lookup_index(i).ok_or("invalid account index")
            }
        }
    }
}
