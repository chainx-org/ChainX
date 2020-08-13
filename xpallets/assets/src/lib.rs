// Copyright 2018-2019 Chainpool.
//! Assets: Handles token asset balances.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod traits;
mod trigger;
pub mod types;

// Substrate
use sp_runtime::traits::{CheckedAdd, CheckedSub, Saturating, Zero};
use sp_std::{collections::btree_map::BTreeMap, prelude::*, result};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Currency, Get, Happened, IsDeadAccount, LockableCurrency, ReservableCurrency},
    StorageDoubleMap,
};
use frame_system::{self as system, ensure_root, ensure_signed};

// ChainX
use chainx_primitives::AssetId;
use xp_runtime::Memo;
use xpallet_support::{debug, ensure_with_errorlog, info};
// re-export
pub use xpallet_assets_registrar::{AssetInfo, Chain};

pub use self::traits::{ChainT, OnAssetChanged};
use self::trigger::AssetChangedTrigger;
pub use self::types::{
    AssetErr, AssetRestriction, AssetRestrictions, AssetType, SignedBalance, TotalAssetInfo,
    WithdrawalLimit,
};

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait Trait: system::Trait + xpallet_assets_registrar::Trait {
    /// Event
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type Currency: ReservableCurrency<Self::AccountId>
        + LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    type OnCreatedAccount: Happened<Self::AccountId>;

    type OnAssetChanged: OnAssetChanged<Self::AccountId, BalanceOf<Self>>;
}

decl_error! {
    /// Error for the Assets Module
    pub enum Error for Module<T: Trait> {
        ///
        InvalidAsset,
        /// Got an overflow after adding
        Overflow,
        /// Balance too low to send value
        InsufficientBalance,
        /// Got an overflow after adding
        TotalAssetOverflow,
        /// Balance too low to send value
        TotalAssetInsufficientBalance,

        ///  Not Allow native asset,
        DenyNativeAsset,
        /// Action is not allowed.
        ActionNotAllowed,
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        Balance = BalanceOf<T>,
        SignedBalance = SignedBalance<T>,
    {
        Move(AssetId, AccountId, AssetType, AccountId, AssetType, Balance),
        Issue(AssetId, AccountId, Balance),
        Destory(AssetId, AccountId, Balance),
        Set(AssetId, AccountId, AssetType, Balance),
        /// change token balance, SignedBalance mark Positive or Negative
        Change(AssetId, AccountId, AssetType, SignedBalance),
        /// set AssetRestrictions for an Asset
        SetRestrictions(AssetId, AssetRestrictions),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// transfer between account
        #[weight = 0]
        pub fn transfer(origin, dest: T::AccountId, #[compact] id: AssetId, #[compact] value: BalanceOf<T>, memo: Memo) -> DispatchResult {
            let transactor = ensure_signed(origin)?;
            debug!("[transfer]|from:{:?}|to:{:?}|id:{:}|value:{:?}|memo:{}", transactor, dest, id, value, memo);
            memo.check_validity()?;
            Self::can_transfer(&id)?;

            Self::move_free_balance(&id, &transactor, &dest, value).map_err::<Error::<T>, _>(Into::into)?;

            Ok(())
        }

        /// for transfer by root
        #[weight = 0]
        pub fn force_transfer(origin, transactor: T::AccountId, dest: T::AccountId, #[compact] id: AssetId, #[compact] value: BalanceOf<T>, memo: Memo) -> DispatchResult {
            ensure_root(origin)?;
            debug!("[force_transfer]|from:{:?}|to:{:?}|id:{:}|value:{:?}|memo:{}", transactor, dest, id, value, memo);
            memo.check_validity()?;
            Self::can_transfer(&id)?;

            Self::move_free_balance(&id, &transactor, &dest, value).map_err::<Error::<T>, _>(Into::into)?;
            Ok(())
        }

        /// set free token for an account
        #[weight = 0]
        pub fn set_balance(origin, who: T::AccountId, #[compact] id: AssetId, balances: BTreeMap<AssetType, BalanceOf<T>>) -> DispatchResult {
            ensure_root(origin)?;
            info!("[set_balance]|set balances by root|who:{:?}|id:{:}|balances_map:{:?}", who, id, balances);
            Self::set_balance_impl(&who, &id, balances)?;
            Ok(())
        }

        #[weight = 0]
        pub fn set_asset_limit(origin, #[compact] id: AssetId, restrictions: AssetRestrictions) -> DispatchResult {
            ensure_root(origin)?;

            Self::set_asset_restrictions(id, restrictions)
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XAssets {
        /// asset extend limit properties, set asset "can do", example, `CanTransfer`, `CanDestroyWithdrawal`
        /// notice if not set AssetRestriction, default is true for this limit
        /// if want let limit make sense, must set false for the limit
        pub AssetRestrictionsOf get(fn asset_restrictions_of): map hasher(twox_64_concat) AssetId => AssetRestrictions;

        /// asset balance for user&asset_id, use btree_map to accept different asset type
        pub AssetBalance get(fn asset_balance):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) AssetId => BTreeMap<AssetType, BalanceOf<T>>;
        /// asset balance for an asset_id, use btree_map to accept different asset type
        pub TotalAssetBalance get(fn total_asset_balance): map hasher(twox_64_concat) AssetId => BTreeMap<AssetType, BalanceOf<T>>;

        /// memo len
        pub MemoLen get(fn memo_len) config(): u32;
    }
    add_extra_genesis {
        config(assets_restrictions): Vec<(AssetId, AssetRestrictions)>;
        config(endowed): BTreeMap<AssetId, Vec<(T::AccountId, BalanceOf<T>)>>;
        build(|config| {
            Module::<T>::endow_assets(&config.endowed);
            Module::<T>::set_restrictions(&config.assets_restrictions);
        })
    }
}

// initialize
impl<T: Trait> Module<T> {
    fn set_restrictions(assets: &[(AssetId, AssetRestrictions)]) {
        for (id, restrictions) in assets.iter() {
            if *id != T::NativeAssetId::get() {
                Self::set_asset_restrictions(*id, *restrictions)
                    .expect("should not fail in genesis, qed");
            }
        }
    }
    fn endow_assets(endowed_accounts: &BTreeMap<AssetId, Vec<(T::AccountId, BalanceOf<T>)>>) {
        for (id, endowed) in endowed_accounts.iter() {
            if *id != T::NativeAssetId::get() {
                for (accountid, value) in endowed.iter() {
                    Self::issue(id, accountid, *value)
                        .expect("asset issuance during the genesis can not fail");
                }
            }
        }
    }
}

// others
impl<T: Trait> Module<T> {
    fn set_asset_restrictions(
        asset_id: AssetId,
        restrictions: AssetRestrictions,
    ) -> DispatchResult {
        // notice use `asset_info_of`, not `asset_online`
        xpallet_assets_registrar::Module::<T>::ensure_assert_exists(&asset_id)?;
        AssetRestrictionsOf::insert(asset_id, restrictions);
        Ok(())
    }
}

impl<T: Trait> Module<T> {
    pub fn ensure_not_native_asset(asset_id: &AssetId) -> DispatchResult {
        ensure!(
            *asset_id != T::NativeAssetId::get(),
            Error::<T>::DenyNativeAsset
        );
        Ok(())
    }
}

// asset related
impl<T: Trait> Module<T> {
    pub fn total_asset_infos() -> BTreeMap<AssetId, TotalAssetInfo<BalanceOf<T>>> {
        xpallet_assets_registrar::Module::<T>::asset_infos()
            .into_iter()
            .filter_map(|(id, info)| {
                if id == T::NativeAssetId::get() {
                    // ignore native asset
                    None
                } else {
                    let data = (
                        id,
                        TotalAssetInfo {
                            info,
                            balance: Self::total_asset_balance(id),
                            is_online: xpallet_assets_registrar::Module::<T>::asset_online(id)
                                .is_some(),
                            restrictions: Self::asset_restrictions_of(id),
                        },
                    );
                    Some(data)
                }
            })
            .collect()
    }

    pub fn valid_assets_of(
        who: &T::AccountId,
    ) -> BTreeMap<AssetId, BTreeMap<AssetType, BalanceOf<T>>> {
        use frame_support::IterableStorageDoubleMap;
        AssetBalance::<T>::iter_prefix(who)
            .filter_map(|(id, map)| {
                xpallet_assets_registrar::Module::<T>::asset_online(id).map(|_| (id, map))
            })
            .collect()
    }

    pub fn can_do(id: &AssetId, limit: AssetRestriction) -> bool {
        !Self::asset_restrictions_of(id).contains(limit)
    }

    // can do wrapper
    #[inline]
    pub fn can_move(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::Move),
            Error::<T>::ActionNotAllowed,
            "this asset do not allow move|id:{:}|action:{:?}",
            id,
            AssetRestriction::Move,
        );
        Ok(())
    }

    #[inline]
    pub fn can_transfer(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::Transfer),
            Error::<T>::ActionNotAllowed,
            "this asset do not allow transfer|id:{:}|action:{:?}",
            id,
            AssetRestriction::Transfer,
        );
        Ok(())
    }

    #[inline]
    pub fn can_destroy_withdrawal(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::DestroyWithdrawal),
            Error::<T>::ActionNotAllowed,
            "this asset do not allow destroy withdrawal|id:{:}|action:{:?}",
            id,
            AssetRestriction::DestroyWithdrawal,
        );
        Ok(())
    }

    #[inline]
    pub fn can_destroy_free(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::DestroyFree),
            Error::<T>::ActionNotAllowed,
            "this asset do not allow destroy free|id:{:}|action:{:?}",
            id,
            AssetRestriction::DestroyFree,
        );
        Ok(())
    }
}

// public read interface
impl<T: Trait> Module<T> {
    pub fn total_issuance(id: &AssetId) -> BalanceOf<T> {
        let map = Self::total_asset_balance(id);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    pub fn total_asset_balance_of(id: &AssetId, type_: AssetType) -> BalanceOf<T> {
        Self::total_asset_balance(id)
            .get(&type_)
            .map(|b| *b)
            .unwrap_or_default()
    }

    pub fn all_type_asset_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        let map = Self::asset_balance(who, id);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    pub fn asset_balance_of(who: &T::AccountId, id: &AssetId, type_: AssetType) -> BalanceOf<T> {
        Self::asset_typed_balance(who, id, type_)
    }

    pub fn usable_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        Self::asset_typed_balance(&who, &id, AssetType::Free)
    }

    pub fn free_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        Self::asset_typed_balance(&who, &id, AssetType::Free)
            + Self::asset_typed_balance(&who, &id, AssetType::Locked)
    }
}

// public write interface
impl<T: Trait> Module<T> {
    pub fn issue(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Module::<T>::ensure_asset_is_valid(id)?;

        let _imbalance = Self::inner_issue(id, who, AssetType::Free, value)?;
        Ok(())
    }

    pub fn destroy(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Module::<T>::ensure_asset_is_valid(id)?;
        Self::can_destroy_withdrawal(id)?;

        let _imbalance = Self::inner_destroy(id, who, AssetType::ReservedWithdrawal, value)?;
        Ok(())
    }

    pub fn destroy_free(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Module::<T>::ensure_asset_is_valid(id)?;
        Self::can_destroy_free(id)?;

        let _imbalance = Self::inner_destroy(id, who, AssetType::Free, value)?;
        Ok(())
    }

    pub fn move_balance(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: BalanceOf<T>,
    ) -> result::Result<(), AssetErr> {
        // check
        Self::ensure_not_native_asset(id).map_err(|_| AssetErr::InvalidAsset)?;
        xpallet_assets_registrar::Module::<T>::ensure_asset_is_valid(id)
            .map_err(|_| AssetErr::InvalidAsset)?;
        Self::can_move(id).map_err(|_| AssetErr::NotAllow)?;

        if value == Zero::zero() {
            // value is zero, do not read storage, no event
            return Ok(());
        }

        let from_balance = Self::asset_typed_balance(from, id, from_type);
        let to_balance = Self::asset_typed_balance(to, id, to_type);

        debug!("[move_balance]|id:{:}|from:{:?}|f_type:{:?}|f_balance:{:?}|to:{:?}|t_type:{:?}|t_balance:{:?}|value:{:?}",
               id, from, from_type, from_balance, to, to_type, to_balance, value);

        // judge balance is enough and test overflow
        let new_from_balance = from_balance
            .checked_sub(&value)
            .ok_or(AssetErr::NotEnough)?;
        let new_to_balance = to_balance.checked_add(&value).ok_or(AssetErr::OverFlow)?;

        // finish basic check, start self check
        if from == to && from_type == to_type {
            // same account, same type, return directly
            // same account also do trigger
            AssetChangedTrigger::<T>::on_move_pre(id, from, from_type, to, to_type, value);
            AssetChangedTrigger::<T>::on_move_post(id, from, from_type, to, to_type, value)?;
            return Ok(());
        }

        // !!! all check pass, start set storage

        AssetChangedTrigger::<T>::on_move_pre(id, from, from_type, to, to_type, value);

        Self::make_type_balance_be(from, id, from_type, new_from_balance);
        Self::make_type_balance_be(to, id, to_type, new_to_balance);

        AssetChangedTrigger::<T>::on_move_post(id, from, from_type, to, to_type, value)?;
        Ok(())
    }

    pub fn move_free_balance(
        id: &AssetId,
        from: &T::AccountId,
        to: &T::AccountId,
        value: BalanceOf<T>,
    ) -> result::Result<(), AssetErr> {
        Self::move_balance(id, from, AssetType::Free, to, AssetType::Free, value)
    }

    pub fn set_balance_impl(
        who: &T::AccountId,
        id: &AssetId,
        balances: BTreeMap<AssetType, BalanceOf<T>>,
    ) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        for (type_, val) in balances.into_iter() {
            let old_val = Self::asset_typed_balance(who, id, type_);
            if old_val == val {
                continue;
            }

            Self::make_type_balance_be(who, id, type_, val);

            AssetChangedTrigger::<T>::on_set_balance(id, who, type_, val)?;
        }
        Ok(())
    }
}

/// token issue destroy reserve/unreserve, it's core function
impl<T: Trait> Module<T> {
    fn asset_typed_balance(who: &T::AccountId, id: &AssetId, type_: AssetType) -> BalanceOf<T> {
        let balance_map = Self::asset_balance(who, id);
        match balance_map.get(&type_) {
            Some(b) => *b,
            None => Zero::zero(),
        }
    }

    fn new_account(who: &T::AccountId) {
        info!("[new_account]|create new account|who:{:?}", who);
        T::OnCreatedAccount::happened(&who)
    }

    fn try_new_account(who: &T::AccountId) {
        // lookup chainx balance
        if system::Module::<T>::is_dead_account(who) {
            Self::new_account(who);
        }
    }

    fn make_type_balance_be(
        who: &T::AccountId,
        id: &AssetId,
        type_: AssetType,
        new_balance: BalanceOf<T>,
    ) {
        let mut original: BalanceOf<T> = Zero::zero();
        // todo change to try_mutate when update to rc5
        let existed = AssetBalance::<T>::contains_key(who, id);
        let exists = AssetBalance::<T>::mutate(
            who,
            id,
            |balance_map: &mut BTreeMap<AssetType, BalanceOf<T>>| {
                if new_balance == Zero::zero() {
                    // remove Zero balance to save space
                    if let Some(old) = balance_map.remove(&type_) {
                        original = old;
                    }
                    // if is_empty(), means not exists
                    !balance_map.is_empty()
                } else {
                    let balance = balance_map.entry(type_).or_default();
                    original = *balance;
                    // modify to new balance
                    *balance = new_balance;
                    true
                }
            },
        );
        if !existed && exists {
            Self::try_new_account(who);
            frame_system::Module::<T>::inc_ref(who);
        } else if existed && !exists {
            frame_system::Module::<T>::dec_ref(who);
            AssetBalance::<T>::remove(who, id);
        }

        TotalAssetBalance::<T>::mutate(id, |total: &mut BTreeMap<AssetType, BalanceOf<T>>| {
            let balance = total.entry(type_).or_default();
            if original <= new_balance {
                *balance = balance.saturating_add(new_balance - original);
            } else {
                *balance = balance.saturating_sub(original - new_balance);
            };
            if *balance == Zero::zero() {
                total.remove(&type_);
            }
        });
    }

    fn inner_issue(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: BalanceOf<T>,
    ) -> result::Result<(), DispatchError> {
        let current = Self::asset_typed_balance(&who, id, type_);

        debug!(
            "[issue]|issue to account|id:{:}|who:{:?}|type:{:?}|current:{:?}|value:{:?}",
            id, who, type_, current, value
        );
        // check
        let new = current.checked_add(&value).ok_or(Error::<T>::Overflow)?;

        AssetChangedTrigger::<T>::on_issue_pre(id, who);

        // set to storage
        Self::make_type_balance_be(who, id, type_, new);

        AssetChangedTrigger::<T>::on_issue_post(id, who, value)?;
        Ok(())
    }

    fn inner_destroy(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: BalanceOf<T>,
    ) -> result::Result<(), DispatchError> {
        let current = Self::asset_typed_balance(&who, id, type_);

        debug!("[destroy_directly]|destroy asset for account|id:{:}|who:{:?}|type:{:?}|current:{:?}|destroy:{:?}",
               id, who, type_, current, value);
        // check
        let new = current
            .checked_sub(&value)
            .ok_or(Error::<T>::InsufficientBalance)?;

        AssetChangedTrigger::<T>::on_destroy_pre(id, who);

        Self::make_type_balance_be(who, id, type_, new);

        AssetChangedTrigger::<T>::on_destroy_post(id, who, value)?;
        Ok(())
    }
}
