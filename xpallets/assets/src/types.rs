// Copyright 2018-2019 Chainpool.

use bitmask::bitmask;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substrate
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_map::BTreeMap, prelude::*, result, slice::Iter};

use frame_support::dispatch::{DispatchError, DispatchResult};
// ChainX
pub use chainx_primitives::{Desc, Memo, Precision, Token};

use super::{BalanceOf, Error, Trait};

const MAX_TOKEN_LEN: usize = 32;
const MAX_DESC_LEN: usize = 128;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum SignedBalance<T: Trait> {
    /// A positive imbalance (funds have been created but none destroyed).
    Positive(BalanceOf<T>),
    /// A negative imbalance (funds have been destroyed but none created).
    Negative(BalanceOf<T>),
}

macro_rules! define_enum {
    (
    $(#[$attr:meta])*
    $Name:ident { $($Variant:ident),* $(,)* }) =>
    {
        $(#[$attr])*
        pub enum $Name {
            $($Variant),*,
        }
        impl $Name {
            pub fn iterator() -> Iter<'static, $Name> {
                static ENUM_ITEMS: &[$Name] = &[$($Name::$Variant),*];
                ENUM_ITEMS.iter()
            }
        }
    }
}

define_enum!(
    #[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Encode, Decode, RuntimeDebug)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    Chain {
        ChainX,
        Bitcoin,
        Ethereum,
        Polkadot,
    }
);

impl Default for Chain {
    fn default() -> Self {
        Chain::ChainX
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct AssetInfo {
    token: Token,
    token_name: Token,
    chain: Chain,
    precision: Precision,
    desc: Desc,
}

impl sp_std::fmt::Debug for AssetInfo {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(
            f,
            "AssetInfo: {{token: {}, token_name: {}, chain: {:?}, precision: {}, desc: {}}}",
            String::from_utf8_lossy(&self.token).into_owned(),
            String::from_utf8_lossy(&self.token_name).into_owned(),
            self.chain,
            self.precision,
            String::from_utf8_lossy(&self.desc).into_owned()
        )
    }
    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl AssetInfo {
    pub fn new<T: Trait>(
        token: Token,
        token_name: Token,
        chain: Chain,
        precision: Precision,
        desc: Desc,
    ) -> result::Result<Self, DispatchError> {
        let a = AssetInfo {
            token,
            token_name,
            chain,
            precision,
            desc,
        };
        a.is_valid::<T>()?;
        Ok(a)
    }
    pub fn is_valid<T: Trait>(&self) -> DispatchResult {
        is_valid_token::<T>(&self.token)?;
        is_valid_token_name::<T>(&self.token_name)?;
        is_valid_desc::<T>(&self.desc)
    }

    pub fn token(&self) -> &Token {
        &self.token
    }
    pub fn token_name(&self) -> &Token {
        &self.token_name
    }
    pub fn chain(&self) -> Chain {
        self.chain
    }
    pub fn desc(&self) -> &Desc {
        &self.desc
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
    pub fn set_desc(&mut self, desc: Desc) {
        self.desc = desc
    }
    pub fn set_token(&mut self, token: Token) {
        self.token = token
    }
    pub fn set_token_name(&mut self, token_name: Token) {
        self.token_name = token_name
    }
}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetType {
    Free,
    ReservedStaking,
    ReservedStakingRevocation,
    ReservedWithdrawal,
    ReservedDexSpot,
    ReservedDexFuture,
    ReservedCurrency,
    ReservedXRC20,
    LockedFee, // LockedFee is special type, normally it must be zero, otherwise there is some error.
}
impl AssetType {
    pub fn iterator() -> Iter<'static, AssetType> {
        static ENUM_ITEMS: &[AssetType] = &[
            AssetType::Free,
            AssetType::ReservedStaking,
            AssetType::ReservedStakingRevocation,
            AssetType::ReservedWithdrawal,
            AssetType::ReservedDexSpot,
            AssetType::ReservedDexFuture,
            AssetType::ReservedCurrency,
            AssetType::ReservedXRC20,
            // notice except LockedFee
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

/// Token can only use ASCII alphanumeric character or "-.|~".
pub fn is_valid_token<T: Trait>(v: &[u8]) -> DispatchResult {
    if v.len() > MAX_TOKEN_LEN || v.is_empty() {
        Err(Error::<T>::InvalidAssetLen)?;
    }
    let is_valid = |c: &u8| -> bool { c.is_ascii_alphanumeric() || "-.|~".as_bytes().contains(c) };
    for c in v.iter() {
        if !is_valid(c) {
            Err(Error::<T>::InvalidChar)?;
        }
    }
    Ok(())
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct WithdrawalLimit<Balance> {
    pub minimal_withdrawal: Balance,
    pub fee: Balance,
}

#[inline]
/// Visible ASCII char [0x20, 0x7E]
fn is_ascii_invisible(c: &u8) -> bool {
    *c < 0x20 || *c > 0x7E
}

/// A valid token name should have a legal length and be visible ASCII chars only.
pub fn is_valid_token_name<T: Trait>(name: &[u8]) -> DispatchResult {
    if name.len() > MAX_TOKEN_LEN || name.is_empty() {
        Err(Error::<T>::InvalidAssetNameLen)?;
    }
    xpallet_support::xss_check(name)?;
    for c in name.iter() {
        if is_ascii_invisible(c) {
            Err(Error::<T>::InvalidAsscii)?;
        }
    }
    Ok(())
}

/// A valid desc should be visible ASCII chars only and not too long.
pub fn is_valid_desc<T: Trait>(desc: &[u8]) -> DispatchResult {
    if desc.len() > MAX_DESC_LEN {
        Err(Error::<T>::InvalidDescLen)?;
    }
    xpallet_support::xss_check(desc)?;
    for c in desc.iter() {
        if is_ascii_invisible(c) {
            Err(Error::<T>::InvalidAsscii)?;
        }
    }
    Ok(())
}
