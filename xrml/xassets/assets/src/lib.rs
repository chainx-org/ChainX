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

extern crate xrml_xaccounts as xaccounts;

//#[cfg(test)]
//mod mock;
//#[cfg(test)]
//mod tests;

pub mod assetdef;
pub mod remark;

use rstd::prelude::*;
use rstd::result::Result as StdResult;
use rstd::slice::Iter;
use runtime_support::dispatch::Result;

use primitives::traits::{As, CheckedAdd, CheckedSub, Zero};
use runtime_support::{StorageMap, StorageValue};
// substrate mod
//use balances::address::Address as RawAddress;
//use balances::EnsureAccountLiquid;
// substrate mod
use system::ensure_signed;

pub use assetdef::{
    is_valid_desc, is_valid_token, Asset, Chain, ChainT, Desc, DescString, Precision, Token,
    TokenString,
};

pub use remark::is_valid_remark;

pub type Address<AccountId, AccountIndex> = balances::address::Address<AccountId, AccountIndex>;

pub trait Trait: balances::Trait + xaccounts::Trait {
    /// Event
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type OnAssetChanged: OnAssetChanged<Self::AccountId, Self::Balance>;
}

pub trait OnAssetChanged<AccountId, Balance> {
    fn on_move(from: &AccountId, to: &AccountId, token: &Token, value: Balance);
    fn on_issue(who: &AccountId, token: &Token, value: Balance);
    fn on_destroy(who: &AccountId, token: &Token, value: Balance);
    fn on_reserve(_who: &AccountId, _token: &Token, _value: Balance) {}
    fn on_unreserve(_who: &AccountId, _token: &Token, _value: Balance) {}
    fn on_set_free(_who: &AccountId, _token: &Token, _value: Balance) {}
    fn on_set_reserved(_who: &AccountId, _token: &Token, _value: Balance) {}
}

impl<AccountId, Balance> OnAssetChanged<AccountId, Balance> for () {
    fn on_move(_: &AccountId, _: &AccountId, _: &Token, _: Balance) {}
    fn on_issue(_: &AccountId, _: &Token, _: Balance) {}
    fn on_destroy(_: &AccountId, _: &Token, _: Balance) {}
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum ReservedType {
    Others,
    Staking,
    AssetsWithdrawal,
    DexSpot,
    DexFuture,
}

impl ReservedType {
    pub fn iterator() -> Iter<'static, ReservedType> {
        static TYPES: [ReservedType; 5] = [
            ReservedType::Others,
            ReservedType::Staking,
            ReservedType::AssetsWithdrawal,
            ReservedType::DexSpot,
            ReservedType::DexFuture,
        ];
        TYPES.into_iter()
    }
}

impl Default for ReservedType {
    fn default() -> Self {
        ReservedType::Others
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
        fn register_asset(asset: Asset, free: T::Balance, reserved: T::Balance) -> Result {
            asset.is_valid()?;
            Self::add_asset(asset, free, reserved)?;
            Ok(())
        }
        fn cancel_asset(token: Token) -> Result {
            is_valid_token(&token)?;
            Self::remove_asset(&token)?;
            Ok(())
        }

        /// set free token for an account
        fn set_asset_free_balance(who: Address<T::AccountId, T::AccountIndex>, token: Token, free: T::Balance) -> Result {
            let who = balances::Module::<T>::lookup(who)?;
            Self::set_free_balance(&who, &token, free)?;
            Ok(())
        }
        /// set reserved token for an account
        fn set_asset_reserved_balance(who: Address<T::AccountId, T::AccountIndex>, token: Token, reserved: T::Balance, res_type: ReservedType) -> Result {
            let who = balances::Module::<T>::lookup(who)?;
            Self::set_reserved_balance(&who, &token, reserved, res_type)?;
            Ok(())
        }

        /// transfer between account
        fn transfer(origin, dest: Address<T::AccountId, T::AccountIndex>, token: Token, value: T::Balance, remark: Vec<u8>) -> Result {
            runtime_io::print("[tokenbalances] transfer");
            let transactor = ensure_signed(origin)?;
            let dest = balances::Module::<T>::lookup(dest)?;

            is_valid_remark::<T>(&remark)?;

            if transactor == dest {
                return Err("transactor and dest account are same");
            }

            Self::init_account(&transactor, &dest);

            Self::move_free_balance(&transactor, &dest, &token, value).map_err(|e| e.info())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XAssets {
//        /// Native asset list len
//        pub NativeAssetsLen get(native_assets_len): map u32 => Token;
        /// Native asset list
        pub NativeAssets get(native_assets): Vec<Token>;
        /// supported cross chain asset list length
        pub CrossChainAssetsLen get(crosschain_assets_len): u32;
        /// supported cross chain asset list
        pub CrossChainAssets: map u32 => Token;

        /// asset info for every token, key is token token
        pub AssetInfo get(asset_info): map Token => Option<(Asset, bool, T::BlockNumber)>;

        /// asset list of a account
        pub CrossChainAssetsOf get(crosschain_assets_of): map T::AccountId => Vec<Token>;

        /// total free token of a token
        pub TotalXFreeBalance get(total_free_balance): map Token => T::Balance;
        /// free x-asset free balance for this accout and token
        pub XFreeBalance: map (T::AccountId, Token) => T::Balance;

        /// total locked token of a token
        pub TotalXReservedBalance get(total_reserved_balance): map Token => T::Balance;
        /// reserved x-asset free balance for this accout and token
        pub XReservedBalance get(reserved_balance): map (T::AccountId, Token, ReservedType) => T::Balance;

        /// price
        pub PCXPriceFor get(pcx_price_for): map Token => Option<T::Balance>;

        /// remark len
        pub RemarkLen get(remark_len) config(): u32;
    }
    add_extra_genesis {
        config(asset_list): Vec<(Asset, Vec<(T::AccountId, u64)>)>;
        config(pcx): (Precision, Desc);
        build(|storage: &mut primitives::StorageMap, _: &mut primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
                use runtime_io::with_externalities;
                use substrate_primitives::Blake2Hasher;
                use primitives::traits::Zero;
                let src_r = storage.clone().build_storage().unwrap().0;
                let mut tmp_storage: runtime_io::TestExternalities<Blake2Hasher> = src_r.into();
                with_externalities(&mut tmp_storage, || {
                    let chainx: Token = <Module<T> as ChainT>::TOKEN.to_vec();
                    let pcx = Asset::new(chainx, Chain::PCX, config.pcx.0, config.pcx.1.clone()).unwrap();
                    Module::<T>::add_asset(pcx, Zero::zero(), Zero::zero()).unwrap();
                });
                let map: primitives::StorageMap = tmp_storage.into();
                storage.extend(map);
        });
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"PCX";
    fn chain() -> Chain {
        Chain::PCX
    }
}

impl<T: Trait> Module<T> {
    // token storage
    /// free balance for a account for a token
    pub fn free_balance(who_token: &(T::AccountId, Token)) -> T::Balance {
        if who_token.1.as_slice() == <Self as ChainT>::TOKEN {
            As::sa(balances::FreeBalance::<T>::get(&who_token.0).as_())
        } else {
            <XFreeBalance<T>>::get(who_token)
        }
    }

    /// The combined token balance of `who` for token
    pub fn total_balance_of(who: &T::AccountId, token: &Token) -> T::Balance {
        let mut v = Self::free_balance(&(who.clone(), token.clone()));
        for t in ReservedType::iterator() {
            v += Self::reserved_balance(&(who.clone(), token.clone(), *t))
        }
        v
    }

    /// total balance of a token
    pub fn total_balance(token: &Token) -> T::Balance {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            As::sa(balances::TotalIssuance::<T>::get().as_())
        } else {
            Self::total_free_balance(token) + Self::total_reserved_balance(token)
        }
    }
}

// asset related
impl<T: Trait> Module<T> {
    /// add an asset into the storage, notice the asset must be valid
    fn add_asset(asset: Asset, free: T::Balance, reserved: T::Balance) -> Result {
        let token = asset.token();
        if AssetInfo::<T>::exists(&token) {
            return Err("already has this token");
        }
        match asset.chain() {
            Chain::PCX => {
                NativeAssets::<T>::mutate(|v| {
                    v.push(token.clone());
                });
            }
            _ => {
                let index = Self::crosschain_assets_len();
                CrossChainAssets::<T>::insert(index, &token);
                CrossChainAssetsLen::<T>::put(index + 1);
            }
        }

        AssetInfo::<T>::insert(&token, (asset, true, system::Module::<T>::block_number()));
        Self::init_asset_balance(&token, free, reserved);
        Ok(())
    }
    fn remove_asset(token: &Token) -> Result {
        if let Some(mut info) = AssetInfo::<T>::get(token) {
            info.1 = false;
            AssetInfo::<T>::insert(token.clone(), info);
            Ok(())
        } else {
            Err("this token dose not register yet or is invalid")
        }
    }

    fn init_asset_balance(token: &Token, free: T::Balance, reserved: T::Balance) {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            return;
        }

        <TotalXFreeBalance<T>>::insert(token, free);
        <TotalXReservedBalance<T>>::insert(token, reserved);
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
            if let Chain::PCX = asset.chain() {
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
        let mut v = Self::native_assets();
        v.extend(Self::crosschain_assets());
        v
    }

    pub fn assets_of(who: &T::AccountId) -> Vec<Token> {
        let mut v = Self::native_assets();
        v.extend(Self::crosschain_assets_of(who));
        v
    }

    pub fn crosschain_assets() -> Vec<Token> {
        let len: u32 = Self::crosschain_assets_len();
        let mut v: Vec<Token> = Vec::new();
        for i in 0..len {
            let token = CrossChainAssets::<T>::get(i);
            v.push(token);
        }
        v
    }

    /// notice don't call this func in runtime
    pub fn valid_assets() -> Vec<Token> {
        Self::assets()
            .into_iter()
            .filter(|t| {
                if let Some(t) = AssetInfo::<T>::get(t) {
                    t.1
                } else {
                    false
                }
            })
            .collect()
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
        if token.as_slice() == <Self as ChainT>::TOKEN {
            return Err("can't issue chainx token");
        }
        Self::is_valid_asset(token)?;

        let key = (who.clone(), token.clone());
        let total_free_token = TotalXFreeBalance::<T>::get(token);
        let free_token = XFreeBalance::<T>::get(&key);
        // check
        let new_free_token = match free_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("free token too high to issue"),
        };
        let new_total_free_token = match total_free_token.checked_add(&value) {
            Some(b) => b,
            None => return Err("total free token too high to issue"),
        };
        // set to storage
        Self::init_asset_for(who, token);

        TotalXFreeBalance::<T>::insert(token, new_total_free_token);
        XFreeBalance::<T>::insert(&key, new_free_token);

        T::OnAssetChanged::on_issue(who, token, value);
        Ok(())
    }

    pub fn destroy(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
        t: ReservedType,
    ) -> Result {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            return Err("can't destroy chainx token");
        }
        Self::is_valid_asset_for(who, token)?;

        // get storage
        let key = (who.clone(), token.clone(), t);
        let total_reserved_token = TotalXReservedBalance::<T>::get(token);
        let reserved_token = XReservedBalance::<T>::get(&key);
        // check
        let new_reserved_token = match reserved_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("reserved token too low to destroy"),
        };
        let new_total_reserved_token = match total_reserved_token.checked_sub(&value) {
            Some(b) => b,
            None => return Err("total reserved token too low to destroy"),
        };
        // set to storage
        TotalXReservedBalance::<T>::insert(token, new_total_reserved_token);
        XReservedBalance::<T>::insert(&key, new_reserved_token);

        T::OnAssetChanged::on_destroy(who, token, value);
        Ok(())
    }

    pub fn reserve(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
        t: ReservedType,
    ) -> Result {
        Self::is_valid_asset_for(who, token)?;

        let key = (who.clone(), token.clone());
        let reserved_key = (who.clone(), token.clone(), t);
        // for chainx
        if token.as_slice() == <Self as ChainT>::TOKEN {
            let free_token: T::Balance = balances::FreeBalance::<T>::get(who);
            let reserved_token = XReservedBalance::<T>::get(&reserved_key);
            let total_reserved_token = TotalXReservedBalance::<T>::get(token);
            let new_free_token = match free_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err("chainx free token too low to reserve"),
            };
            let new_reserved_token = match reserved_token.checked_add(&value) {
                Some(b) => b,
                None => return Err("chainx reserved token too high to reserve"),
            };
            let new_total_reserved_token = match total_reserved_token.checked_add(&value) {
                Some(b) => b,
                None => return Err("chainx total reserved token too high to reserve"),
            };
            // do not call reserve in balance
            //            balances::Module::<T>::reserve(who, value)?;
            balances::Module::<T>::set_free_balance(who, new_free_token);
            XReservedBalance::<T>::insert(reserved_key, new_reserved_token);
            TotalXReservedBalance::<T>::insert(token, new_total_reserved_token);
        } else {
            // for other token
            // get from storage
            let total_free_token = TotalXFreeBalance::<T>::get(token);
            let total_reserved_token = TotalXReservedBalance::<T>::get(token);
            let free_token = XFreeBalance::<T>::get(&key);
            let reserved_token = XReservedBalance::<T>::get(&reserved_key);
            // test overflow
            let new_free_token = match free_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err("free token too low to reserve"),
            };
            let new_reserved_token = match reserved_token.checked_add(&value) {
                Some(b) => b,
                None => return Err("reserved token too high to reserve"),
            };
            let new_total_free_token = match total_free_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err("total free token too low to reserve"),
            };
            let new_total_reserved_token = match total_reserved_token.checked_add(&value) {
                Some(b) => b,
                None => return Err("total reserved token too high to reserve"),
            };
            // set to storage
            TotalXFreeBalance::<T>::insert(token, new_total_free_token);
            TotalXReservedBalance::<T>::insert(token, new_total_reserved_token);
            XFreeBalance::<T>::insert(&key, new_free_token);
            XReservedBalance::<T>::insert(&reserved_key, new_reserved_token);
        }
        T::OnAssetChanged::on_reserve(who, token, value);
        Ok(())
    }

    pub fn unreserve(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
        t: ReservedType,
    ) -> Result {
        Self::is_valid_asset_for(who, token)?;

        let key = (who.clone(), token.clone());
        let reserved_key = (who.clone(), token.clone(), t);
        // for chainx
        if token.as_slice() == <Self as ChainT>::TOKEN {
            let free_token: T::Balance = balances::FreeBalance::<T>::get(who);
            let reserved_token = XReservedBalance::<T>::get(&reserved_key);
            let total_reserved_token = TotalXReservedBalance::<T>::get(token);
            let new_free_token = match free_token.checked_add(&value) {
                Some(b) => b,
                None => return Err("chainx free token too high to unreserve"),
            };
            let new_reserved_token = match reserved_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err("chainx reserved token too low to unreserve"),
            };
            let new_total_reserved_token = match total_reserved_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err("chainx total reserved token too low to unreserve"),
            };
            // do not call unreserve in balance
            //            balances::Module::<T>::unreserve(who, value);
            balances::Module::<T>::set_free_balance(who, new_free_token);
            XReservedBalance::<T>::insert(reserved_key, new_reserved_token);
            TotalXReservedBalance::<T>::insert(token, new_total_reserved_token);
        } else {
            // for other token
            // get from storage
            let total_free_token = TotalXFreeBalance::<T>::get(token);
            let total_reserved_token = TotalXReservedBalance::<T>::get(token);
            let free_token = XFreeBalance::<T>::get(&key);
            let reserved_token = XReservedBalance::<T>::get(&reserved_key);
            // test overflow
            let new_free_token = match free_token.checked_add(&value) {
                Some(b) => b,
                None => return Err("free token too high to unreserve"),
            };
            let new_reserved_token = match reserved_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err("reserved token too low to unreserve"),
            };
            let new_total_free_token = match total_free_token.checked_add(&value) {
                Some(b) => b,
                None => return Err("total free token too high to unreserve"),
            };
            let new_total_reserved_token = match total_reserved_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err("total reserved token too low to unreserve"),
            };
            // set to storage
            TotalXFreeBalance::<T>::insert(token, new_total_free_token);
            TotalXReservedBalance::<T>::insert(token, new_total_reserved_token);
            XFreeBalance::<T>::insert(&key, new_free_token);
            XReservedBalance::<T>::insert(&reserved_key, new_reserved_token);
        }
        T::OnAssetChanged::on_unreserve(who, token, value);
        Ok(())
    }

    pub fn init_account(from: &T::AccountId, to: &T::AccountId) {
        if let None = xaccounts::Module::<T>::account_relationships(to) {
            if balances::FreeBalance::<T>::exists(to) == false {
                xaccounts::AccountRelationships::<T>::insert(to, from);
                balances::Module::<T>::set_free_balance_creating(&to, Zero::zero());
            }
        }
    }

    pub fn move_free_balance(
        from: &T::AccountId,
        to: &T::AccountId,
        token: &Token,
        value: T::Balance,
    ) -> StdResult<(), TokenErr> {
        Self::is_valid_asset_for(from, token).map_err(|_| TokenErr::InvalidToken)?;

        // for chainx
        if token.as_slice() == <Self as ChainT>::TOKEN {
            let from_token: T::Balance = balances::Module::<T>::free_balance(from);
            let to_token: T::Balance = balances::Module::<T>::free_balance(to);

            let new_from_token = match from_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err(TokenErr::NotEnough),
            };
            let new_to_token = match to_token.checked_add(&value) {
                Some(b) => b,
                None => return Err(TokenErr::OverFlow),
            };
//            balances::FreeBalance::<T>::insert(from, new_from_token);
//            balances::FreeBalance::<T>::insert(to, new_to_token);
            balances::Module::<T>::set_free_balance(from, new_from_token);
            balances::Module::<T>::set_free_balance(to, new_to_token);
        } else {
            Self::init_asset_for(to, token);
            let key_from = (from.clone(), token.clone());
            let key_to = (to.clone(), token.clone());

            let from_token: T::Balance = XFreeBalance::<T>::get(&key_from);
            let to_token: T::Balance = XFreeBalance::<T>::get(&key_to);

            let new_from_token = match from_token.checked_sub(&value) {
                Some(b) => b,
                None => return Err(TokenErr::NotEnough),
            };
            let new_to_token = match to_token.checked_add(&value) {
                Some(b) => b,
                None => return Err(TokenErr::OverFlow),
            };

            XFreeBalance::<T>::insert(key_from, new_from_token);
            XFreeBalance::<T>::insert(key_to, new_to_token);
        }
        T::OnAssetChanged::on_move(from, to, token, value);
        Ok(())
    }

    pub fn set_free_balance(who: &T::AccountId, token: &Token, free: T::Balance) -> Result {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            balances::Module::<T>::set_free_balance(&who, free);
        } else {
            let key = (who.clone(), token.clone());
            let old_free = XFreeBalance::<T>::get(&key);
            let old_total_free = TotalXFreeBalance::<T>::get(token);
            if old_free == free {
                return Err("some value for free token");
            }
            let new_total_free = if free > old_free {
                match free.checked_sub(&old_free) {
                    None => return Err("free token too low to sub value"),
                    Some(b) => match old_total_free.checked_add(&b) {
                        None => return Err("old total free token too high to add value"),
                        Some(new) => new,
                    },
                }
            } else {
                match old_free.checked_sub(&free) {
                    None => return Err("old free token too low to sub value"),
                    Some(b) => match old_total_free.checked_sub(&b) {
                        None => return Err("old total free token too low to sub value"),
                        Some(new) => new,
                    },
                }
            };
            TotalXFreeBalance::<T>::insert(token, new_total_free);
            XFreeBalance::<T>::insert(key, free);
        }
        T::OnAssetChanged::on_set_free(who, token, free);
        Ok(())
    }

    pub fn set_reserved_balance(
        who: &T::AccountId,
        token: &Token,
        reserved: T::Balance,
        res_type: ReservedType,
    ) -> Result {
        let key = (who.clone(), token.clone(), res_type);
        let old_reserved = XReservedBalance::<T>::get(&key);
        let old_total_reserved = TotalXReservedBalance::<T>::get(token);

        if old_reserved == reserved {
            return Err("some value for reserved token");
        }

        let new_total_reserved = if reserved > old_reserved {
            match reserved.checked_sub(&old_reserved) {
                None => return Err("reserved token too low to sub value"),
                Some(b) => match old_total_reserved.checked_add(&b) {
                    None => return Err("old total reserved token too high to add value"),
                    Some(new) => new,
                },
            }
        } else {
            match old_reserved.checked_sub(&reserved) {
                None => return Err("old reserved token too low to sub value"),
                Some(b) => match old_total_reserved.checked_sub(&b) {
                    None => return Err("old total reserved token too high to sub value"),
                    Some(new) => new,
                },
            }
        };
        TotalXReservedBalance::<T>::insert(token, new_total_reserved);
        XReservedBalance::<T>::insert(key, reserved);

        T::OnAssetChanged::on_set_reserved(who, token, reserved);
        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum TokenErr {
    NotEnough,
    OverFlow,
    InvalidToken,
    InvalidAccount,
}

impl TokenErr {
    pub fn info(&self) -> &'static str {
        match *self {
            TokenErr::NotEnough => "free token too low",
            TokenErr::OverFlow => "overflow for this value",
            TokenErr::InvalidToken => "not a valid token for this account",
            TokenErr::InvalidAccount => "Account Locked",
        }
    }
}

// wrapper for balances module
impl<T: Trait> Module<T> {
    pub fn pcx_free_balance(who: &T::AccountId) -> T::Balance {
        balances::Module::<T>::free_balance(who)
    }

    pub fn pcx_total_balance(who: &T::AccountId) -> T::Balance {
        Self::total_balance_of(who, &<Self as ChainT>::TOKEN.to_vec())
    }

    pub fn pcx_set_free_balance(who: &T::AccountId, value: T::Balance) {
        balances::Module::<T>::set_free_balance(who, value);
    }

    pub fn pcx_reward(who: &T::AccountId, value: T::Balance) -> Result {
        balances::Module::<T>::reward(who, value)
    }

    pub fn pcx_staking_reserve(who: &T::AccountId, value: T::Balance) -> Result {
        Self::reserve(
            who,
            &<Self as ChainT>::TOKEN.to_vec(),
            value,
            ReservedType::Staking,
        )
    }

    pub fn pcx_staking_unreserve(who: &T::AccountId, value: T::Balance) -> Result {
        Self::unreserve(
            who,
            &<Self as ChainT>::TOKEN.to_vec(),
            value,
            ReservedType::Staking,
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
