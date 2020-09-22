// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use bitflags::bitflags;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substrate
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_map::BTreeMap, prelude::*, slice::Iter};

// ChainX
pub use chainx_primitives::{Decimals, Desc, Token};
use xpallet_assets_registrar::AssetInfo;

use super::{Error, Trait};
use frame_support::traits::LockIdentifier;

const ASSET_TYPES: [AssetType; 6] = [
    AssetType::Usable,
    AssetType::Locked,
    AssetType::Reserved,
    AssetType::ReservedWithdrawal,
    AssetType::ReservedDexSpot,
    AssetType::ReservedXRC20,
];

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetType {
    Usable,
    Locked,
    Reserved,
    ReservedWithdrawal,
    ReservedDexSpot,
    ReservedXRC20,
}

impl AssetType {
    pub fn iter() -> Iter<'static, AssetType> {
        ASSET_TYPES.iter()
    }
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Usable
    }
}

bitflags! {
    /// Restrictions for asset operations.
    #[derive(Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct AssetRestrictions: u32 {
        const Move                = 1 << 0;
        const Transfer            = 1 << 1;
        const Deposit             = 1 << 2;
        const Withdraw            = 1 << 3;
        const DestroyWithdrawal   = 1 << 4;
        const DestroyUsable       = 1 << 5;
    }
}

impl Default for AssetRestrictions {
    fn default() -> Self {
        AssetRestrictions::empty()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TotalAssetInfo<Balance> {
    pub info: AssetInfo,
    pub balance: BTreeMap<AssetType, Balance>,
    pub is_online: bool,
    pub restrictions: AssetRestrictions,
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetErr {
    NotEnough,
    OverFlow,
    TotalAssetNotEnough,
    TotalAssetOverFlow,
    InvalidAsset,
    NotAllow,
}

impl<T: Trait> From<AssetErr> for Error<T> {
    fn from(err: AssetErr) -> Self {
        match err {
            AssetErr::NotEnough => Error::<T>::InsufficientBalance,
            AssetErr::OverFlow => Error::<T>::Overflow,
            AssetErr::TotalAssetNotEnough => Error::<T>::TotalAssetInsufficientBalance,
            AssetErr::TotalAssetOverFlow => Error::<T>::TotalAssetOverflow,
            AssetErr::InvalidAsset => Error::<T>::InvalidAsset,
            AssetErr::NotAllow => Error::<T>::ActionNotAllowed,
        }
    }
}

/// A single lock on a balance. There can be many of these on an account and
/// they "overlap", so the same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct BalanceLock<Balance> {
    /// An identifier for this lock. Only one lock may be in existence for each
    /// identifier.
    pub id: LockIdentifier,
    /// The amount which the free balance may not drop below when this lock is
    /// in effect.
    pub amount: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct WithdrawalLimit<Balance> {
    pub minimal_withdrawal: Balance,
    pub fee: Balance,
}
