// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Assets: Handles token asset balances.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity, clippy::transmute_ptr_to_ptr)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_multicurrency;

mod default_weight;
mod multicurrency;
pub mod traits;
mod trigger;
pub mod types;

use sp_std::{
    collections::btree_map::BTreeMap,
    convert::{TryFrom, TryInto},
    prelude::*,
};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Currency, Get, Happened, IsDeadAccount, LockableCurrency, ReservableCurrency},
    weights::Weight,
    Parameter, StorageDoubleMap,
};
use frame_system::{ensure_root, ensure_signed};
use orml_traits::arithmetic::{Signed, SimpleArithmetic};
use sp_runtime::traits::{
    CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, Saturating, StaticLookup, Zero,
};

use chainx_primitives::AssetId;
use xp_logging::{debug, error, info};
pub use xpallet_assets_registrar::{AssetInfo, Chain};
use xpallet_support::traits::TreasuryAccount;

pub use self::traits::{ChainT, OnAssetChanged};
use self::trigger::AssetChangedTrigger;
pub use self::types::{
    AssetErr, AssetRestrictions, AssetType, BalanceLock, TotalAssetInfo, WithdrawalLimit,
};

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

/// Weight information for extrinsics in this pallet.
pub trait WeightInfo {
    fn transfer() -> Weight;
    fn force_transfer() -> Weight;
    fn set_balance(n: u32) -> Weight;
    fn set_asset_limit() -> Weight;
}

/// The module's config trait.
///
/// `frame_system::Trait` should always be included in our implied traits.
pub trait Trait: xpallet_assets_registrar::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type Currency: ReservableCurrency<Self::AccountId>
        + LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// The amount type, should be signed version of `Balance`
    type Amount: Parameter
        + Member
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Signed
        + SimpleArithmetic
        + TryInto<BalanceOf<Self>>
        + TryFrom<BalanceOf<Self>>;

    type TreasuryAccount: TreasuryAccount<Self::AccountId>;

    type OnCreatedAccount: Happened<Self::AccountId>;

    type OnAssetChanged: OnAssetChanged<Self::AccountId, BalanceOf<Self>>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
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
        /// Failed because liquidity restrictions due to locking
        LiquidityRestrictions,
        /// Cannot convert Amount into Balance type
        AmountIntoBalanceFailed,
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
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>,
    {
        /// Some balances of an asset was moved from one to another. [asset_id, from, from_type, to, to_type, amount]
        Moved(AssetId, AccountId, AssetType, AccountId, AssetType, Balance),
        /// New balances of an asset were issued. [asset_id, receiver, amount]
        Issued(AssetId, AccountId, Balance),
        /// Some balances of an asset were destoryed. [asset_id, who, amount]
        Destroyed(AssetId, AccountId, Balance),
        /// Set asset balance of an account by root. [asset_id, who, asset_type, amount]
        SetBalance(AssetId, AccountId, AssetType, Balance),
        /// Set restrictions for an asset by root. [asset_id, assets_restrictions]
        SetRestrictions(AssetId, AssetRestrictions),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XAssets {
        /// asset extend limit properties, set asset "can do", example, `CanTransfer`, `CanDestroyWithdrawal`
        /// notice if not set AssetRestriction, default is true for this limit
        /// if want let limit make sense, must set false for the limit
        pub AssetRestrictionsOf get(fn asset_restrictions_of):
            map hasher(twox_64_concat) AssetId => AssetRestrictions;

        /// asset balance for user&asset_id, use btree_map to accept different asset type
        pub AssetBalance get(fn asset_balance):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) AssetId
            => BTreeMap<AssetType, BalanceOf<T>>;

        /// Any liquidity locks of a token type under an account.
        /// NOTE: Should only be accessed when setting, changing and freeing a lock.
        pub Locks get(fn locks):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) AssetId
            => Vec<BalanceLock<BalanceOf<T>>>;

        /// asset balance for an asset_id, use btree_map to accept different asset type
        pub TotalAssetBalance get(fn total_asset_balance):
            map hasher(twox_64_concat) AssetId => BTreeMap<AssetType, BalanceOf<T>>;
    }
    add_extra_genesis {
        config(assets_restrictions): Vec<(AssetId, AssetRestrictions)>;
        config(endowed): BTreeMap<AssetId, Vec<(T::AccountId, BalanceOf<T>)>>;
        build(|config| {
            for (id, endowed) in &config.endowed {
                if *id != T::NativeAssetId::get() {
                    for (accountid, value) in endowed.iter() {
                        Module::<T>::issue(id, accountid, *value)
                            .expect("asset issuance during the genesis can not fail");
                    }
                }
            }
            for (id, restrictions) in &config.assets_restrictions {
                if *id != T::NativeAssetId::get() {
                    Module::<T>::set_asset_restrictions(*id, *restrictions)
                        .expect("should not fail in genesis, qed");
                }
            }
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// transfer between account
        #[weight = 0]
        pub fn transfer(
            origin,
            dest: <T::Lookup as StaticLookup>::Source,
            #[compact] id: AssetId,
            #[compact] value: BalanceOf<T>
        ) -> DispatchResult {
            let transactor = ensure_signed(origin)?;
            let dest = T::Lookup::lookup(dest)?;
            debug!("[transfer] from:{:?}, to:{:?}, id:{}, value:{:?}", transactor, dest, id, value);
            Self::can_transfer(&id)?;

            Self::move_usable_balance(&id, &transactor, &dest, value).map_err::<Error::<T>, _>(Into::into)?;

            Ok(())
        }

        /// for transfer by root
        #[weight = 0]
        pub fn force_transfer(
            origin,
            transactor: <T::Lookup as StaticLookup>::Source,
            dest: <T::Lookup as StaticLookup>::Source,
            #[compact] id: AssetId,
            #[compact] value: BalanceOf<T>
        ) -> DispatchResult {
            ensure_root(origin)?;

            let transactor = T::Lookup::lookup(transactor)?;
            let dest = T::Lookup::lookup(dest)?;
            debug!("[force_transfer] from:{:?}, to:{:?}, id:{}, value:{:?}", transactor, dest, id, value);
            Self::can_transfer(&id)?;
            Self::move_usable_balance(&id, &transactor, &dest, value).map_err::<Error::<T>, _>(Into::into)?;
            Ok(())
        }

        /// set free token for an account
        #[weight = 0]
        pub fn set_balance(
            origin,
            who: <T::Lookup as StaticLookup>::Source,
            #[compact] id: AssetId,
            balances: BTreeMap<AssetType, BalanceOf<T>>
        ) -> DispatchResult {
            ensure_root(origin)?;

            let who = T::Lookup::lookup(who)?;
            info!("[set_balance] Set balance by root, who:{:?}, id:{}, balances:{:?}", who, id, balances);
            Self::set_balance_impl(&who, &id, balances)?;
            Ok(())
        }

        #[weight = <T as Trait>::WeightInfo::set_asset_limit()]
        pub fn set_asset_limit(origin, #[compact] id: AssetId, restrictions: AssetRestrictions) -> DispatchResult {
            ensure_root(origin)?;
            Self::set_asset_restrictions(id, restrictions)
        }
    }
}

// others
impl<T: Trait> Module<T> {
    fn set_asset_restrictions(
        asset_id: AssetId,
        restrictions: AssetRestrictions,
    ) -> DispatchResult {
        xpallet_assets_registrar::Module::<T>::ensure_asset_exists(&asset_id)?;
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
                            is_online: xpallet_assets_registrar::Module::<T>::is_online(&id),
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
            .filter(|(id, _)| xpallet_assets_registrar::Module::<T>::asset_online(id))
            .collect()
    }

    pub fn can_do(id: &AssetId, limit: AssetRestrictions) -> bool {
        !Self::asset_restrictions_of(id).contains(limit)
    }

    // can do wrapper
    #[inline]
    pub fn can_move(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::MOVE) {
            error!("Not allowed to move asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
        Ok(())
    }

    #[inline]
    pub fn can_transfer(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::TRANSFER) {
            error!("Not allowed to transfer asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
        Ok(())
    }

    #[inline]
    pub fn can_destroy_withdrawal(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::DESTROY_WITHDRAWAL) {
            error!("Not allowed to destroy withdrawal asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
        Ok(())
    }

    #[inline]
    pub fn can_destroy_usable(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::DESTROY_USABLE) {
            error!("Not allowed to destroy usable asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
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
            .copied()
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
        Self::asset_typed_balance(&who, &id, AssetType::Usable)
    }

    pub fn locked_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        Self::asset_typed_balance(&who, &id, AssetType::Locked)
    }
}

// public write interface
impl<T: Trait> Module<T> {
    /// Sets the free balance of `who` without sanity checks and triggering the asset changed hook.
    #[cfg(feature = "std")]
    pub fn force_set_free_balance(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) {
        Self::make_type_balance_be(who, id, AssetType::Usable, value);
    }

    pub fn issue(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Module::<T>::ensure_asset_is_valid(id)?;

        let _imbalance = Self::inner_issue(id, who, AssetType::Usable, value)?;
        Ok(())
    }

    pub fn destroy_reserved_withdrawal(
        id: &AssetId,
        who: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Module::<T>::ensure_asset_is_valid(id)?;
        Self::can_destroy_withdrawal(id)?;

        Self::inner_destroy(id, who, AssetType::ReservedWithdrawal, value)?;
        Ok(())
    }

    pub fn destroy_usable(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Module::<T>::ensure_asset_is_valid(id)?;
        Self::can_destroy_usable(id)?;

        Self::inner_destroy(id, who, AssetType::Usable, value)?;
        Ok(())
    }

    pub fn move_balance(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: BalanceOf<T>,
    ) -> Result<(), AssetErr> {
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

        debug!(
            "[move_balance] id:{}, from:[who:{:?}, type:{:?}, balance:{:?}], to:[who:{:?}, type:{:?}, balance:{:?}], value:{:?}",
            id, from, from_type, from_balance, to, to_type, to_balance, value
        );

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

    pub fn move_usable_balance(
        id: &AssetId,
        from: &T::AccountId,
        to: &T::AccountId,
        value: BalanceOf<T>,
    ) -> Result<(), AssetErr> {
        Self::move_balance(id, from, AssetType::Usable, to, AssetType::Usable, value)
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
    /// Returns the balance of `who` given `asset_id` and `ty`.
    fn asset_typed_balance(who: &T::AccountId, asset_id: &AssetId, ty: AssetType) -> BalanceOf<T> {
        Self::asset_balance(who, asset_id)
            .get(&ty)
            .copied()
            .unwrap_or_default()
    }

    fn new_account(who: &T::AccountId) {
        info!("[new_account] account:{:?}", who);
        T::OnCreatedAccount::happened(&who)
    }

    fn try_new_account(who: &T::AccountId) {
        // lookup chainx balance
        if frame_system::Module::<T>::is_dead_account(who) {
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
    ) -> Result<(), DispatchError> {
        let current = Self::asset_typed_balance(&who, id, type_);

        debug!(
            "[issue] account:{:?}, asset:[id:{}, type:{:?}, current:{:?}, issue:{:?}]",
            who, id, type_, current, value
        );

        let new = current.checked_add(&value).ok_or(Error::<T>::Overflow)?;

        AssetChangedTrigger::<T>::on_issue_pre(id, who);

        Self::make_type_balance_be(who, id, type_, new);

        AssetChangedTrigger::<T>::on_issue_post(id, who, value)?;
        Ok(())
    }

    fn inner_destroy(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        let current = Self::asset_typed_balance(&who, id, type_);

        debug!(
            "[destroy] account:{:?}, asset:[id:{}, type:{:?}, current:{:?}, destroy:{:?}]",
            who, id, type_, current, value
        );

        let new = current
            .checked_sub(&value)
            .ok_or(Error::<T>::InsufficientBalance)?;

        AssetChangedTrigger::<T>::on_destroy_pre(id, who);

        Self::make_type_balance_be(who, id, type_, new);

        AssetChangedTrigger::<T>::on_destroy_post(id, who, value)?;
        Ok(())
    }

    fn update_locks(currency_id: AssetId, who: &T::AccountId, locks: &[BalanceLock<BalanceOf<T>>]) {
        // update locked balance
        if let Some(max_locked) = locks.iter().map(|lock| lock.amount).max() {
            use sp_std::cmp::Ordering;
            let current_locked = Self::asset_balance_of(who, &currency_id, AssetType::Locked);

            let result = match max_locked.cmp(&current_locked) {
                Ordering::Greater => {
                    // new lock more than current locked, move usable to locked
                    Self::move_balance(
                        &currency_id,
                        who,
                        AssetType::Usable,
                        who,
                        AssetType::Locked,
                        max_locked - current_locked,
                    )
                }
                Ordering::Less => {
                    // new lock less then current locked, release locked to usable
                    Self::move_balance(
                        &currency_id,
                        who,
                        AssetType::Locked,
                        who,
                        AssetType::Usable,
                        current_locked - max_locked,
                    )
                }
                Ordering::Equal => {
                    // if max_locked == locked, need do nothing
                    Ok(())
                }
            };
            if let Err(err) = result {
                // should not fail, for set lock need to check free_balance, free_balance = usable + free
                error!(
                    "[update_locks] Should not be failed when move asset (usable <=> locked), \
                    who:{:?}, asset:[id:{}, max_locked:{:?}, current_locked:{:?}], err:{:?}",
                    who, currency_id, max_locked, current_locked, err
                );
            }
        }

        // update locks
        if locks.is_empty() {
            <Locks<T>>::remove(who, currency_id);
        } else {
            <Locks<T>>::insert(who, currency_id, locks);
        }
    }
}
