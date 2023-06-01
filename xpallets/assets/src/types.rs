// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use bitflags::bitflags;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substrate
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_map::BTreeMap, prelude::*, slice::Iter};

// ChainX
pub use chainx_primitives::{Decimals, Desc, Token};
use xpallet_assets_registrar::AssetInfo;

use frame_support::traits::LockIdentifier;

use crate::{Config, Error};

const ASSET_TYPES: [AssetType; 5] = [
    AssetType::Usable,
    AssetType::Locked,
    AssetType::Reserved,
    AssetType::ReservedWithdrawal,
    AssetType::ReservedDexSpot,
];

/// Concrete type of non-native asset balance.
///
/// NOTE: The native token also reserves an AssetId in this module, but it's
/// handle by Balances runtime module in fact.
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetType {
    /// Free balance.
    Usable,
    /// Placeholder for the future use.
    ///
    /// Unused for now.
    Locked,
    /// General reserved balance.
    ///
    /// Unused for now.
    Reserved,
    /// Reserved balance when an account redeems its bridged asset.
    ReservedWithdrawal,
    /// Reserved balance for creating order in DEX.
    ReservedDexSpot,
}

impl AssetType {
    /// Returns an iterator of all asset types.
    pub fn iter() -> Iter<'static, AssetType> {
        ASSET_TYPES.iter()
    }
}

impl Default for AssetType {
    fn default() -> Self {
        Self::Usable
    }
}

bitflags! {
    /// Restrictions for asset operations.
    #[derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct AssetRestrictions: u32 {
        const MOVE                = 1 << 0;
        const TRANSFER            = 1 << 1;
        const DEPOSIT             = 1 << 2;
        const WITHDRAW            = 1 << 3;
        const DESTROY_WITHDRAWAL  = 1 << 4;
        const DESTROY_USABLE      = 1 << 5;
    }
}

impl Default for AssetRestrictions {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TotalAssetInfo<Balance> {
    pub info: AssetInfo,
    pub balance: BTreeMap<AssetType, Balance>,
    pub is_online: bool,
    pub restrictions: AssetRestrictions,
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetErr {
    NotEnough,
    OverFlow,
    TotalAssetNotEnough,
    TotalAssetOverFlow,
    InvalidAsset,
    NotAllow,
}

impl<T: Config> From<AssetErr> for Error<T> {
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BalanceLock<Balance> {
    /// An identifier for this lock. Only one lock may be in existence for each
    /// identifier.
    pub id: LockIdentifier,
    /// The amount which the free balance may not drop below when this lock is
    /// in effect.
    pub amount: Balance,
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct WithdrawalLimit<Balance> {
    pub minimal_withdrawal: Balance,
    pub fee: Balance,
}
