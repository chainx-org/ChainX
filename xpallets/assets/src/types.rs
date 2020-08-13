// Copyright 2018-2019 Chainpool.

use bitmask::bitmask;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substrate
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_map::BTreeMap, prelude::*, slice::Iter};

// ChainX
pub use chainx_primitives::{Desc, Precision, Token};
pub use xp_runtime::Memo;
use xpallet_assets_metadata::AssetInfo;

use super::{BalanceOf, Error, Trait};

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum SignedBalance<T: Trait> {
    /// A positive imbalance (funds have been created but none destroyed).
    Positive(BalanceOf<T>),
    /// A negative imbalance (funds have been destroyed but none created).
    Negative(BalanceOf<T>),
}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetType {
    Free,
    Locked,
    ReservedWithdrawal,
    ReservedDexSpot,
    ReservedXRC20,
}
impl AssetType {
    pub fn iterator() -> Iter<'static, AssetType> {
        static ENUM_ITEMS: &[AssetType] = &[
            AssetType::Free,
            AssetType::Locked,
            AssetType::ReservedWithdrawal,
            AssetType::ReservedDexSpot,
            AssetType::ReservedXRC20,
        ];
        ENUM_ITEMS.iter()
    }
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Free
    }
}

bitmask! {
    ///
    #[derive(Encode, Decode)]
    #[cfg_attr(not(feature = "std"), derive(RuntimeDebug))]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub mask AssetRestrictions: u32 where
    ///
    #[derive(Encode, Decode)]
    #[cfg_attr(not(feature = "std"), derive(RuntimeDebug))]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    flags AssetRestriction {
        Move                = 1 << 0,
        Transfer            = 1 << 1,
        Deposit             = 1 << 2,
        Withdraw            = 1 << 3,
        DestroyWithdrawal   = 1 << 4,
        DestroyFree         = 1 << 5,
    }
}

impl Default for AssetRestrictions {
    fn default() -> Self {
        AssetRestrictions::none()
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct WithdrawalLimit<Balance> {
    pub minimal_withdrawal: Balance,
    pub fee: Balance,
}
