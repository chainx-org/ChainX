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

mod multicurrency;
pub mod traits;
mod trigger;
pub mod types;
pub mod weights;

use sp_std::{
    collections::btree_map::BTreeMap,
    convert::{TryFrom, TryInto},
};

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    inherent::Vec,
    log::{debug, error, info},
    traits::{Currency, Get, HandleLifetime, LockableCurrency, ReservableCurrency},
    Parameter,
};

use frame_system::{ensure_root, ensure_signed, AccountInfo};
use orml_traits::arithmetic::{Signed, SimpleArithmetic};
use sp_runtime::traits::{CheckedAdd, CheckedSub, Saturating, StaticLookup, Zero};

use self::trigger::AssetChangedTrigger;
use chainx_primitives::AssetId;
use xpallet_support::traits::TreasuryAccount;

pub use self::traits::{ChainT, OnAssetChanged};
pub use self::types::{
    AssetErr, AssetRestrictions, AssetType, BalanceLock, TotalAssetInfo, WithdrawalLimit,
};
pub use self::weights::WeightInfo;
pub use xpallet_assets_registrar::{AssetInfo, Chain};

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// The pallet's config trait.
    ///
    /// `frame_system::Config` should always be included in our implied traits.
    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets_registrar::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The native currency.
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

        /// The treasury account.
        type TreasuryAccount: TreasuryAccount<Self::AccountId>;

        /// The hook for doing something on the event of creating an account.
        type OnCreatedAccount: HandleLifetime<Self::AccountId>;

        /// The hook triggered whenever the asset balance of an account is changed.
        type OnAssetChanged: OnAssetChanged<Self::AccountId, BalanceOf<Self>>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// transfer between two accounts
        #[pallet::weight(0)]
        pub fn transfer(
            origin: OriginFor<T>,
            dest: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] id: AssetId,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let transactor = ensure_signed(origin)?;
            let dest = T::Lookup::lookup(dest)?;
            debug!(target: "runtime::assets", "[transfer] from:{:?}, to:{:?}, id:{}, value:{:?}", transactor, dest, id, value);
            Self::can_transfer(&id)?;

            Self::move_usable_balance(&id, &transactor, &dest, value)
                .map_err::<Error<T>, _>(Into::into)?;

            Ok(())
        }

        /// transfer method reserved for root(sudo)
        #[pallet::weight(0)]
        pub fn force_transfer(
            origin: OriginFor<T>,
            transactor: <T::Lookup as StaticLookup>::Source,
            dest: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] id: AssetId,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let transactor = T::Lookup::lookup(transactor)?;
            let dest = T::Lookup::lookup(dest)?;
            debug!(target: "runtime::assets", "[force_transfer] from:{:?}, to:{:?}, id:{}, value:{:?}", transactor, dest, id, value);
            Self::can_transfer(&id)?;
            Self::move_usable_balance(&id, &transactor, &dest, value)
                .map_err::<Error<T>, _>(Into::into)?;
            Ok(())
        }

        /// set free token for an account
        #[pallet::weight(0)]
        pub fn set_balance(
            origin: OriginFor<T>,
            who: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] id: AssetId,
            balances: BTreeMap<AssetType, BalanceOf<T>>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let who = T::Lookup::lookup(who)?;
            info!(target: "runtime::assets", "[set_balance] Set balance by root, who:{:?}, id:{}, balances:{:?}", who, id, balances);
            Self::set_balance_impl(&who, &id, balances)?;
            Ok(())
        }

        /// asset restriction method reserved for root
        #[pallet::weight(<T as Config>::WeightInfo::set_asset_limit())]
        pub fn set_asset_limit(
            origin: OriginFor<T>,
            #[pallet::compact] id: AssetId,
            restrictions: AssetRestrictions,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::set_asset_restrictions(id, restrictions)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        /// Some balances of an asset was moved from one to another. [asset_id, from, from_type, to, to_type, amount]
        Moved(
            AssetId,
            T::AccountId,
            AssetType,
            T::AccountId,
            AssetType,
            BalanceOf<T>,
        ),
        /// New balances of an asset were issued. [asset_id, receiver, amount]
        Issued(AssetId, T::AccountId, BalanceOf<T>),
        /// Some balances of an asset were destoryed. [asset_id, who, amount]
        Destroyed(AssetId, T::AccountId, BalanceOf<T>),
        /// Set asset balance of an account by root. [asset_id, who, asset_type, amount]
        BalanceSet(AssetId, T::AccountId, AssetType, BalanceOf<T>),
    }

    /// Error for the Assets Pallet
    #[pallet::error]
    pub enum Error<T> {
        /// Got and Invalid Asset
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
        /// Account still has active reserved
        StillHasActiveReserved,
    }

    /// asset extend limit properties, set asset "can do", example, `CanTransfer`, `CanDestroyWithdrawal`
    /// notice if not set AssetRestriction, default is true for this limit
    /// if want let limit make sense, must set false for the limit
    #[pallet::storage]
    #[pallet::getter(fn asset_restrictions_of)]
    pub type AssetRestrictionsOf<T: Config> =
        StorageMap<_, Twox64Concat, AssetId, AssetRestrictions, ValueQuery>;

    /// asset balance for user&asset_id, use btree_map to accept different asset type
    #[pallet::storage]
    #[pallet::getter(fn asset_balance)]
    pub type AssetBalance<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Twox64Concat,
        AssetId,
        BTreeMap<AssetType, BalanceOf<T>>,
        ValueQuery,
    >;

    /// Any liquidity locks of a token type under an account.
    /// NOTE: Should only be accessed when setting, changing and freeing a lock.
    #[pallet::storage]
    #[pallet::getter(fn locks)]
    pub type Locks<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Twox64Concat,
        AssetId,
        Vec<BalanceLock<BalanceOf<T>>>,
        ValueQuery,
    >;

    /// asset balance for an asset_id, use btree_map to accept different asset type
    #[pallet::storage]
    #[pallet::getter(fn total_asset_balance)]
    pub type TotalAssetBalance<T: Config> =
        StorageMap<_, Twox64Concat, AssetId, BTreeMap<AssetType, BalanceOf<T>>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub assets_restrictions: Vec<(AssetId, AssetRestrictions)>,
        pub endowed: BTreeMap<AssetId, Vec<(T::AccountId, BalanceOf<T>)>>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                assets_restrictions: Default::default(),
                endowed: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let extra_genesis_builder: fn(&Self) = |config| {
                for (id, endowed) in &config.endowed {
                    if *id != T::NativeAssetId::get() {
                        for (accountid, value) in endowed.iter() {
                            Pallet::<T>::issue(id, accountid, *value)
                                .expect("asset issuance during the genesis can not fail");
                        }
                    }
                }
                for (id, restrictions) in &config.assets_restrictions {
                    if *id != T::NativeAssetId::get() {
                        Pallet::<T>::set_asset_restrictions(*id, *restrictions)
                            .expect("should not fail in genesis, qed");
                    }
                }
            };
            extra_genesis_builder(self);
        }
    }
}

impl<T: Config> Pallet<T> {
    fn set_asset_restrictions(
        asset_id: AssetId,
        restrictions: AssetRestrictions,
    ) -> DispatchResult {
        xpallet_assets_registrar::Pallet::<T>::ensure_asset_exists(&asset_id)?;
        AssetRestrictionsOf::<T>::insert(asset_id, restrictions);
        Ok(())
    }

    pub fn ensure_not_native_asset(asset_id: &AssetId) -> DispatchResult {
        ensure!(
            *asset_id != T::NativeAssetId::get(),
            Error::<T>::DenyNativeAsset
        );
        Ok(())
    }

    /// Asset related
    ///
    /// Returns a map of all registered assets by far.
    pub fn total_asset_infos() -> BTreeMap<AssetId, TotalAssetInfo<BalanceOf<T>>> {
        xpallet_assets_registrar::Pallet::<T>::asset_infos()
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
                            is_online: xpallet_assets_registrar::Pallet::<T>::is_online(&id),
                            restrictions: Self::asset_restrictions_of(id),
                        },
                    );
                    Some(data)
                }
            })
            .collect()
    }

    /// Returns the invalid asset info of `who`.
    pub fn valid_assets_of(
        who: &T::AccountId,
    ) -> BTreeMap<AssetId, BTreeMap<AssetType, BalanceOf<T>>> {
        AssetBalance::<T>::iter_prefix(who)
            .filter(|(id, _)| xpallet_assets_registrar::Pallet::<T>::asset_online(id))
            .collect()
    }

    /// Returns whether `restriction` is applied for given asset `id`.
    pub fn can_do(id: &AssetId, restriction: AssetRestrictions) -> bool {
        !Self::asset_restrictions_of(id).contains(restriction)
    }

    // can do wrapper
    #[inline]
    pub fn can_move(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::MOVE) {
            error!(target: "runtime::assets", "Not allowed to move asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
        Ok(())
    }

    #[inline]
    pub fn can_transfer(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::TRANSFER) {
            error!(target: "runtime::assets", "Not allowed to transfer asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
        Ok(())
    }

    #[inline]
    pub fn can_destroy_withdrawal(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::DESTROY_WITHDRAWAL) {
            error!(target: "runtime::assets", "Not allowed to destroy withdrawal asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
        Ok(())
    }

    #[inline]
    pub fn can_destroy_usable(id: &AssetId) -> DispatchResult {
        if !Self::can_do(id, AssetRestrictions::DESTROY_USABLE) {
            error!(target: "runtime::assets", "Not allowed to destroy usable asset, id:{}", id);
            return Err(Error::<T>::ActionNotAllowed.into());
        }
        Ok(())
    }

    /// Public read functions.
    ///
    /// Returns the total issuance of asset `id` by far.
    pub fn total_issuance(id: &AssetId) -> BalanceOf<T> {
        let map = Self::total_asset_balance(id);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    /// Returns the total balance of asset `id` given the specific asset type `ty`.
    pub fn total_asset_balance_of(id: &AssetId, ty: AssetType) -> BalanceOf<T> {
        Self::total_asset_balance(id)
            .get(&ty)
            .copied()
            .unwrap_or_default()
    }

    /// Returns the sum of all kinds of `who`'s balances given asset `id`.
    pub fn all_type_asset_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        let map = Self::asset_balance(who, id);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    /// Returns the balance of `who` given the asset `id` and type `ty`.
    pub fn asset_balance_of(who: &T::AccountId, id: &AssetId, ty: AssetType) -> BalanceOf<T> {
        Self::asset_typed_balance(who, id, ty)
    }

    /// Returns the free balance of `who` for asset `id`.
    pub fn usable_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        Self::asset_typed_balance(who, id, AssetType::Usable)
    }

    pub fn locked_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        Self::asset_typed_balance(who, id, AssetType::Locked)
    }

    pub fn total_reserved_balance(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        use AssetType::{Reserved, ReservedDexSpot, ReservedWithdrawal};

        let total_balances = Self::asset_balance(who, id);
        let balance_for = |ty: AssetType| total_balances.get(&ty).copied().unwrap_or_default();

        balance_for(Reserved) + balance_for(ReservedWithdrawal) + balance_for(ReservedDexSpot)
    }

    /// Sets the free balance of `who` without sanity checks and triggering the asset changed hook.
    #[cfg(feature = "std")]
    pub fn force_set_free_balance(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) {
        Self::make_type_balance_be(who, id, AssetType::Usable, value);
    }

    /// Increases the Usable balance of `who` given the asset `id` by this `value`.
    pub fn issue(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Pallet::<T>::ensure_asset_is_valid(id)?;

        let _imbalance = Self::inner_issue(id, who, AssetType::Usable, value)?;
        Ok(())
    }

    pub fn destroy_reserved_withdrawal(
        id: &AssetId,
        who: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Pallet::<T>::ensure_asset_is_valid(id)?;
        Self::can_destroy_withdrawal(id)?;

        Self::inner_destroy(id, who, AssetType::ReservedWithdrawal, value)?;
        Ok(())
    }

    pub fn destroy_usable(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::ensure_not_native_asset(id)?;
        xpallet_assets_registrar::Pallet::<T>::ensure_asset_is_valid(id)?;
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
        Self::ensure_not_native_asset(id).map_err(|_| AssetErr::InvalidAsset)?;
        xpallet_assets_registrar::Pallet::<T>::ensure_asset_is_valid(id)
            .map_err(|_| AssetErr::InvalidAsset)?;
        Self::can_move(id).map_err(|_| AssetErr::NotAllow)?;

        if value == Zero::zero() {
            // value is zero, do not read storage, no event
            return Ok(());
        }

        let from_balance = Self::asset_typed_balance(from, id, from_type);
        let to_balance = Self::asset_typed_balance(to, id, to_type);

        debug!(
            target: "runtime::assets",
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

    /// Token issue destroy reserve/unreserved, it's core function
    ///
    /// Returns the balance of `who` given `asset_id` and `ty`.
    fn asset_typed_balance(who: &T::AccountId, asset_id: &AssetId, ty: AssetType) -> BalanceOf<T> {
        Self::asset_balance(who, asset_id)
            .get(&ty)
            .copied()
            .unwrap_or_default()
    }

    fn new_account(who: &T::AccountId) {
        info!(target: "runtime::assets", "[new_account] account:{:?}", who);
        // FIXME: handle the result properly.
        let _ = T::OnCreatedAccount::created(who);
    }

    fn is_dead_account(who: &T::AccountId) -> bool {
        let AccountInfo {
            providers,
            consumers,
            ..
        } = frame_system::pallet::Account::<T>::get(who);
        providers.is_zero() && consumers.is_zero()
    }

    fn try_new_account(who: &T::AccountId) {
        // lookup chainx balance
        if Self::is_dead_account(who) {
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
            // FIXME: handle the result properly
            let _ = frame_system::Pallet::<T>::inc_consumers(who);
        } else if existed && !exists {
            frame_system::Pallet::<T>::dec_consumers(who);
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
            target: "runtime::assets",
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
            target: "runtime::assets",
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
                    target: "runtime::assets",
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
