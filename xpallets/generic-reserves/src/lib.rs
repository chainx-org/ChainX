#![cfg_attr(not(feature = "std"), no_std)]

//! This crate implements the feature of tracking various kinds of reserved balances,
//! providing an unified way of managing the reserves of native coin and the assets
//! injected externally abatracted.
//!
//! The origin Balances system of Substrate only tracks the total reserved balance
//! of an account. You are unable to know how many exact balances are retained for a
//! particular reason, that is why this Module is made for.

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageMap,
    traits::Get,
};
use frame_system::{self as system, ensure_signed};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{Convert, SaturatedConversion, Saturating, Zero};
use sp_runtime::RuntimeDebug;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

use chainx_primitives::AssetId;
use xpallet_assets::AssetType;

pub trait Trait: pallet_balances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum NativeReservedType {
    StakingBonded,
    StakingBondedWithdrawal,
    DexSpot,
}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ReservedType {
    Native(NativeReservedType),
    ExtAsset(AssetType),
}

decl_storage! {
    trait Store for Module<T: Trait> as XGenericReserves {
        /// All kinds of reserves of an account.
        pub Reserves get(fn reserves):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) AssetId => BTreeMap<ReservedType, T::Balance>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as pallet_balances::Trait>::Balance,
    {
        /// The staker has been rewarded by this amount. `AccountId` is the stash account.
        ReservedBalance(AccountId, Balance),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Zero amount
        ZeroBalance,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;
    }
}

// TODO: impl GenericReservableCurrency trait
impl<T: Trait> Module<T> {
    /// Reserve the `value` of asset `asset_id` for the reason `ty`.
    ///
    /// This operation will move some Free balances of `who` to the reserves.
    pub fn generic_reserve(
        who: &T::AccountId,
        asset_id: AssetId,
        value: T::Balance,
        ty: ReservedType,
    ) -> Result<(), Error<T>> {
        Ok(())
    }

    ////////////////////////////////////////////////////
    // Native coin reserves.
    ////////////////////////////////////////////////////

    /// Moves the `value` of the native coin of `who` from Free to the native reserved type `ty`.
    pub fn reserve_native(
        who: &T::AccountId,
        value: T::Balance,
        ty: NativeReservedType,
    ) -> Result<(), Error<T>> {
        Ok(())
    }

    ///
    pub fn unreserve_native(
        who: &T::AccountId,
        value: T::Balance,
        from_ty: NativeReservedType,
    ) -> Result<(), Error<T>> {
        Ok(())
    }

    /// Move the `value` of the native coin of `who` from reserved type `from_ty` to `to_ty`.
    pub fn move_reserved_native(
        who: &T::AccountId,
        value: T::Balance,
        from_ty: NativeReservedType,
        to_ty: NativeReservedType,
    ) -> Result<(), Error<T>> {
        Ok(())
    }

    /// Returns the balance of `who` for the reserved type `ty`.
    pub fn native_reserved_of(who: &T::AccountId, ty: NativeReservedType) -> T::Balance {
        todo!()
    }

    /// Returns the sum of all kinds of native reserved balances of `who`.
    pub fn total_native_reserved_of(who: &T::AccountId) -> T::Balance {
        todo!()
    }

    ////////////////////////////////////////////////////
    // Non-Native assets/tokens reserves.
    ////////////////////////////////////////////////////

    /// Reserves the `value` of non-native asset `asset_id` of `who` for the reason `ty`.
    pub fn reserve_asset(
        who: &T::AccountId,
        asset_id: AssetId,
        value: T::Balance,
        ty: AssetType,
    ) -> Result<(), Error<T>> {
        Ok(())
    }

    ///
    pub fn unreserve_asset(
        who: &T::AccountId,
        asset_id: AssetId,
        value: T::Balance,
        ty: AssetType,
    ) -> Result<(), Error<T>> {
        Ok(())
    }

    ///
    pub fn move_reserved_asset(
        who: &T::AccountId,
        asset_id: AssetId,
        value: T::Balance,
        from_ty: AssetType,
        to_ty: AssetType,
    ) -> Result<(), Error<T>> {
        Ok(())
    }

    ///
    pub fn reserved_asset_of(who: &T::AccountId, asset_id: AssetId, ty: AssetType) -> T::Balance {
        todo!()
    }

    pub fn total_reserved_asset_of(who: &T::AccountId, asset_id: AssetId) -> T::Balance {
        todo!()
    }
}
