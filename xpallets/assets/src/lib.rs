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
    traits::Currency,
};
use frame_system::{self as system, ensure_root, ensure_signed};

// ChainX
use chainx_primitives::{AssetId, Desc, Memo, Token};
use xpallet_support::{debug, ensure_with_errorlog, info};

use self::trigger::AssetChangedTrigger;

pub use self::traits::{ChainT, OnAssetChanged, OnAssetRegisterOrRevoke};
pub use self::types::{
    is_valid_desc, is_valid_token, AssetErr, AssetInfo, AssetRestriction, AssetRestrictions,
    AssetType, Chain, SignedBalance, TotalAssetInfo, WithdrawalLimit,
};
use frame_support::traits::IsDeadAccount;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
// pub type PositiveImbalanceOf<T> =
// <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::NegativeImbalance;
// pub type NegativeImbalanceOf<T> =
// <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::NegativeImbalance;
// pub type SignedImbalance<T> = frame_support::traits::SignedImbalance<BalanceOf<T>, PositiveImbalance<T>>;

pub trait Trait: system::Trait {
    /// Event
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type Currency: Currency<Self::AccountId>;

    type OnAssetChanged: OnAssetChanged<Self::AccountId, BalanceOf<Self>>;

    type OnAssetRegisterOrRevoke: OnAssetRegisterOrRevoke;
}

decl_error! {
    /// Error for the Assets Module
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
        /// Got an overflow after adding
        Overflow,
        /// Balance too low to send value
        InsufficientBalance,
        /// Got an overflow after adding
        TotalAssetOverflow,
        /// Balance too low to send value
        TotalAssetInsufficientBalance,

        /// Free asset type is not allowed.
        FreeTypeNotAllowed,
        /// ChainX token is not allowed.
        PcxNotAllowed,
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
        Register(AssetId, bool),
        Revoke(AssetId),

        Move(AssetId, AccountId, AssetType, AccountId, AssetType, Balance),
        Issue(AssetId, AccountId, Balance),
        Destory(AssetId, AccountId, Balance),
        Set(AssetId, AccountId, AssetType, Balance),

        /// change token balance, SignedBalance mark Positive or Negative
        Change(AssetId, AccountId, AssetType, SignedBalance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// register_asset to module, should allow by root
        #[weight = 0]
        pub fn register_asset(
            origin,
            #[compact] asset_id: AssetId,
            asset: AssetInfo,
            restrictions: AssetRestrictions,
            is_online: bool,
            has_mining_rights: bool
        ) -> DispatchResult {
            ensure_root(origin)?;
            asset.is_valid::<T>()?;
            info!("[register_asset]|id:{:}|{:?}|is_online:{:}|has_mining_rights:{:}", asset_id, asset, is_online, has_mining_rights);

            Self::add_asset(asset_id, asset, restrictions)?;

            T::OnAssetRegisterOrRevoke::on_register(&asset_id, has_mining_rights)?;
            Self::deposit_event(RawEvent::Register(asset_id, has_mining_rights));

            if !is_online {
                let _ = Self::revoke_asset(frame_system::RawOrigin::Root.into(), asset_id.into());
            }
            Ok(())
        }

        /// revoke asset, mark this asset is invalid
        #[weight = 0]
        pub fn revoke_asset(origin, #[compact] id: AssetId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);
            Self::remove_asset(&id)?;

            T::OnAssetRegisterOrRevoke::on_revoke(&id)?;
            Self::deposit_event(RawEvent::Revoke(id));
            Ok(())
        }

        /// set free token for an account
        #[weight = 0]
        pub fn set_balance(origin, who: T::AccountId, #[compact] id: AssetId, balances: BTreeMap<AssetType, BalanceOf<T>>) -> DispatchResult {
            ensure_root(origin)?;
            info!("[set_balance]|set balances by root|who:{:?}|id:{:}|balances_map:{:?}", who, id, balances);
            Self::set_balance_by_root(&who, &id, balances)?;
            Ok(())
        }

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

        #[weight = 0]
        pub fn modify_asset_limit(origin, #[compact] id: AssetId, restriction: AssetRestriction, can_do: bool) -> DispatchResult {
            ensure_root(origin)?;
            // notice use `asset_info_of`, not `asset_online`
            ensure!(Self::asset_info_of(id).is_some(), Error::<T>::InvalidAsset);

            AssetRestrictionsOf::mutate(id, |current| {
                if can_do {
                    current.set(restriction);
                } else {
                    current.unset(restriction);
                }
            });
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XAssets {
        /// Asset id list for Chain, different Chain has different id list
        pub AssetIdsOf get(fn asset_ids_of): map hasher(twox_64_concat) Chain => Vec<AssetId>;

        /// asset info for every asset, key is asset id
        pub AssetInfoOf get(fn asset_info_of): map hasher(twox_64_concat) AssetId => Option<AssetInfo>;
        pub AssetOnline get(fn asset_online): map hasher(twox_64_concat) AssetId => Option<()>;
        pub AssetRegisteredBlock get(fn asset_registered_block): map hasher(twox_64_concat) AssetId => T::BlockNumber;
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
        config(assets): Vec<(AssetId, AssetInfo, AssetRestrictions, bool, bool)>;
        config(endowed): BTreeMap<AssetId, Vec<(T::AccountId, BalanceOf<T>)>>;
        build(|config| {
            Module::<T>::initialize_assets(&config.assets, &config.endowed);
        })
    }
}

// impl<T: Trait> ChainT<BalanceOf<T>> for Module<T> {
//     const ASSET_ID: AssetId = xpallet_protocol::PCX;
//     fn chain() -> Chain {
//         Chain::ChainX
//     }
//     fn withdrawal_limit(_: &AssetId) -> result::Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
//         Err(Error::<T>::ActionNotAllowed)?
//     }
// }

impl<T: Trait> Module<T> {
    fn initialize_assets(
        assets: &Vec<(AssetId, AssetInfo, AssetRestrictions, bool, bool)>,
        endowed_accounts: &BTreeMap<AssetId, Vec<(T::AccountId, BalanceOf<T>)>>,
    ) {
        for (id, asset, restrictions, is_online, has_mining_rights) in assets {
            Self::register_asset(
                frame_system::RawOrigin::Root.into(),
                (*id).into(),
                asset.clone(),
                restrictions.clone(),
                *is_online,
                *has_mining_rights,
            )
            .expect("asset registeration during the genesis can not fail");
        }

        for (id, endowed) in endowed_accounts.iter() {
            for (accountid, value) in endowed.iter() {
                Self::issue(id, accountid, *value)
                    .expect("asset issuance during the genesis can not fail");
            }
        }
    }

    pub fn should_not_free_type(type_: AssetType) -> DispatchResult {
        if type_ == AssetType::Free {
            Err(Error::<T>::FreeTypeNotAllowed)?;
        }
        Ok(())
    }

    // pub fn should_not_chainx(id: &AssetId) -> DispatchResult {
    //     if *id == <Self as ChainT<_>>::ASSET_ID {
    //         Err(Error::<T>::PcxNotAllowed)?;
    //     }
    //     Ok(())
    // }
}

// asset related
impl<T: Trait> Module<T> {
    /// add an asset into the storage, notice the asset must be valid
    fn add_asset(id: AssetId, asset: AssetInfo, restrictions: AssetRestrictions) -> DispatchResult {
        let chain = asset.chain();
        if Self::asset_info_of(&id).is_some() {
            Err(Error::<T>::AlreadyExistentToken)?;
        }

        AssetInfoOf::insert(&id, asset);
        AssetRestrictionsOf::insert(&id, restrictions);
        AssetOnline::insert(&id, ());

        AssetRegisteredBlock::<T>::insert(&id, system::Module::<T>::block_number());

        AssetIdsOf::mutate(chain, |v| {
            if !v.contains(&id) {
                v.push(id.clone());
            }
        });
        Ok(())
    }

    fn remove_asset(id: &AssetId) -> DispatchResult {
        AssetOnline::remove(id);
        Ok(())
    }

    pub fn asset_ids() -> Vec<AssetId> {
        let mut v = Vec::new();
        for i in Chain::iterator() {
            v.extend(Self::asset_ids_of(i));
        }
        v
    }

    pub fn total_asset_infos() -> BTreeMap<AssetId, TotalAssetInfo<BalanceOf<T>>> {
        use frame_support::IterableStorageMap;
        AssetInfoOf::iter()
            .map(|(id, info)| {
                (
                    id,
                    TotalAssetInfo {
                        info,
                        balance: Self::total_asset_balance(id),
                        is_online: Self::asset_online(id).is_some(),
                        restrictions: Self::asset_restrictions_of(id),
                    },
                )
            })
            .collect()
    }

    pub fn valid_asset_ids() -> Vec<AssetId> {
        Self::asset_ids()
            .into_iter()
            .filter(|id| Self::asset_online(id).is_some())
            .collect()
    }

    pub fn valid_assets_of(
        who: &T::AccountId,
    ) -> BTreeMap<AssetId, BTreeMap<AssetType, BalanceOf<T>>> {
        use frame_support::IterableStorageDoubleMap;
        AssetBalance::<T>::iter_prefix(who)
            .filter_map(|(id, map)| Self::asset_online(id).map(|_| (id, map)))
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

/// token issue destroy reserve/unreserve, it's core function
impl<T: Trait> Module<T> {
    pub fn all_type_total_asset_balance(id: &AssetId) -> BalanceOf<T> {
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
        Self::asset_type_balance(who, id, type_)
    }

    pub fn free_balance_of(who: &T::AccountId, id: &AssetId) -> BalanceOf<T> {
        Self::asset_type_balance(&who, &id, AssetType::Free)
    }

    fn asset_type_balance(who: &T::AccountId, id: &AssetId, type_: AssetType) -> BalanceOf<T> {
        let balance_map = Self::asset_balance(who, id);
        match balance_map.get(&type_) {
            Some(b) => *b,
            None => Zero::zero(),
        }
    }

    pub fn issue(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);

        // may set storage inner
        Self::try_new_account(&who);

        let _imbalance = Self::inner_issue(id, who, AssetType::Free, value)?;
        Ok(())
    }

    pub fn destroy(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);
        Self::can_destroy_withdrawal(id)?;

        let _imbalance = Self::inner_destroy(id, who, AssetType::ReservedWithdrawal, value)?;
        Ok(())
    }

    pub fn destroy_free(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);
        Self::can_destroy_free(id)?;

        let _imbalance = Self::inner_destroy(id, who, AssetType::Free, value)?;
        Ok(())
    }

    fn new_account(who: &T::AccountId) {
        info!("[new_account]|create new account|who:{:?}", who);
        system::Module::<T>::on_created_account(who.clone())
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
        AssetBalance::<T>::mutate(
            who,
            id,
            |balance_map: &mut BTreeMap<AssetType, BalanceOf<T>>| {
                if new_balance == Zero::zero() {
                    // remove Zero balance to save space
                    if let Some(old) = balance_map.remove(&type_) {
                        original = old;
                    }
                } else {
                    let balance = balance_map.entry(type_).or_default();
                    original = *balance;
                    // modify to new balance
                    *balance = new_balance;
                }
            },
        );
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
        let current = Self::asset_type_balance(&who, id, type_);

        debug!(
            "[issue]|issue to account|id:{:}|who:{:?}|type:{:?}|current:{:?}|value:{:?}",
            id, who, type_, current, value
        );
        // check
        let new = current.checked_add(&value).ok_or(Error::<T>::Overflow)?;

        AssetChangedTrigger::<T>::on_issue_pre(id, who);

        // set to storage
        Self::make_type_balance_be(who, id, type_, new);
        // let positive = if let SignedImbalance::Positive(p) = imbalance {
        //     p
        // } else {
        //     // Impossible, but be defensive.
        //     PositiveImbalanceOf::<T>::new(Zero::zero(), *id, type_)
        // };

        AssetChangedTrigger::<T>::on_issue_post(id, who, value)?;
        Ok(())
    }

    fn inner_destroy(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: BalanceOf<T>,
    ) -> result::Result<(), DispatchError> {
        let current = Self::asset_type_balance(&who, id, type_);

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

    pub fn move_balance(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: BalanceOf<T>,
    ) -> result::Result<(), AssetErr> {
        // check
        ensure!(Self::asset_online(id).is_some(), AssetErr::InvalidAsset);
        Self::can_move(id).map_err(|_| AssetErr::NotAllow)?;

        if value == Zero::zero() {
            // value is zero, do not read storage, no event
            return Ok(());
        }

        let from_balance = Self::asset_type_balance(from, id, from_type);
        let to_balance = Self::asset_type_balance(to, id, to_type);

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
        // for account to set storage
        Self::try_new_account(to);

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

    pub fn set_balance_by_root(
        who: &T::AccountId,
        id: &AssetId,
        balances: BTreeMap<AssetType, BalanceOf<T>>,
    ) -> DispatchResult {
        for (type_, val) in balances.into_iter() {
            let old_val = Self::asset_type_balance(who, id, type_);
            if old_val == val {
                continue;
            }

            Self::make_type_balance_be(who, id, type_, val);

            AssetChangedTrigger::<T>::on_set_balance(id, who, type_, val)?;
        }
        Ok(())
    }
}
