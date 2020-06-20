// Copyright 2018-2019 Chainpool.

use bitmask::bitmask;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substrate
use sp_runtime::{
    traits::{Saturating, Zero},
    RuntimeDebug,
};
use sp_std::{prelude::*, result, slice::Iter};

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    traits::{Imbalance, SignedImbalance},
};
// ChainX
pub use chainx_primitives::{Desc, Memo, Precision, Token};

use super::traits::ChainT;
use super::{Error, Module, Trait};

pub use self::imbalances::{NegativeImbalance, PositiveImbalance};

const MAX_TOKEN_LEN: usize = 32;
const MAX_DESC_LEN: usize = 128;

pub type SignedImbalanceT<T> = SignedImbalance<<T as Trait>::Balance, PositiveImbalance<T>>;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum SignedBalance<T: Trait> {
    /// A positive imbalance (funds have been created but none destroyed).
    Positive(T::Balance),
    /// A negative imbalance (funds have been destroyed but none created).
    Negative(T::Balance),
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
pub struct Asset {
    token: Token,
    token_name: Token,
    chain: Chain,
    precision: Precision,
    desc: Desc,
}

impl sp_std::fmt::Debug for Asset {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(
            f,
            "Asset: {{token: {}, token_name: {}, chain: {:?}, precision: {}, desc: {}}}",
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

impl Asset {
    pub fn new<T: Trait>(
        token: Token,
        token_name: Token,
        chain: Chain,
        precision: Precision,
        desc: Desc,
    ) -> result::Result<Self, DispatchError> {
        let a = Asset {
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

    pub fn token(&self) -> Token {
        self.token.clone()
    }
    pub fn token_name(&self) -> Token {
        self.token_name.clone()
    }
    pub fn chain(&self) -> Chain {
        self.chain
    }
    pub fn desc(&self) -> Desc {
        self.desc.clone()
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
    pub fn set_desc(&mut self, desc: Desc) {
        self.desc = desc
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

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetErr {
    NotEnough,
    OverFlow,
    TotalAssetNotEnough,
    TotalAssetOverFlow,
    InvalidToken,
    NotAllow,
}

impl AssetErr {
    pub fn to_err<T: Trait>(self) -> Error<T> {
        self.into()
    }
}

impl<T: Trait> From<AssetErr> for Error<T> {
    fn from(err: AssetErr) -> Self {
        match err {
            AssetErr::NotEnough => Error::<T>::InsufficientBalance,
            AssetErr::OverFlow => Error::<T>::Overflow,
            AssetErr::TotalAssetNotEnough => Error::<T>::TotalAssetInsufficientBalance,
            AssetErr::TotalAssetOverFlow => Error::<T>::TotalAssetOverflow,
            AssetErr::InvalidToken => Error::<T>::InvalidToken,
            AssetErr::NotAllow => Error::<T>::NotAllowAction,
        }
    }
}

/// Token can only use ASCII alphanumeric character or "-.|~".
pub fn is_valid_token<T: Trait>(v: &[u8]) -> DispatchResult {
    if v.len() > MAX_TOKEN_LEN || v.is_empty() {
        Err(Error::<T>::InvalidTokenLen)?;
    }
    let is_valid = |c: &u8| -> bool { c.is_ascii_alphanumeric() || "-.|~".as_bytes().contains(c) };
    for c in v.iter() {
        if !is_valid(c) {
            Err(Error::<T>::InvalidChar)?;
        }
    }
    Ok(())
}

#[inline]
/// Visible ASCII char [0x20, 0x7E]
fn is_ascii_invisible(c: &u8) -> bool {
    *c < 0x20 || *c > 0x7E
}

/// A valid token name should have a legal length and be visible ASCII chars only.
pub fn is_valid_token_name<T: Trait>(name: &[u8]) -> DispatchResult {
    if name.len() > MAX_TOKEN_LEN || name.is_empty() {
        Err(Error::<T>::InvalidTokenNameLen)?;
    }
    xrml_support::xss_check(name)?;
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
    xrml_support::xss_check(desc)?;
    for c in desc.iter() {
        if is_ascii_invisible(c) {
            Err(Error::<T>::InvalidAsscii)?;
        }
    }
    Ok(())
}

/// A valid memo should have a legal length and be xss proof.
pub fn is_valid_memo<T: Trait>(memo: &Memo) -> DispatchResult {
    if memo.len() as u32 > Module::<T>::memo_len() {
        Err(Error::<T>::InvalidMemoLen)?;
    }
    xrml_support::xss_check(memo)
}

mod imbalances {
    use frame_support::{traits::TryDrop, StorageMap};
    use sp_std::mem;

    use chainx_primitives::Token;

    use super::{result, AssetType, ChainT, Imbalance, Saturating, Zero};
    use crate::{Module, TotalAssetBalance, Trait};

    /// Opaque, move-only struct with private fields that serves as a token denoting that
    /// funds have been created without any equal and opposite accounting.
    #[must_use]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq))]
    pub struct PositiveImbalance<T: Trait>(T::Balance, Token, AssetType);
    impl<T: Trait> PositiveImbalance<T> {
        /// Create a new positive imbalance from a balance.
        pub fn new(amount: T::Balance, token: Token, type_: AssetType) -> Self {
            PositiveImbalance(amount, token, type_)
        }
    }

    /// Opaque, move-only struct with private fields that serves as a token denoting that
    /// funds have been destroyed without any equal and opposite accounting.
    #[must_use]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq))]
    pub struct NegativeImbalance<T: Trait>(T::Balance, Token, AssetType);
    impl<T: Trait> NegativeImbalance<T> {
        /// Create a new negative imbalance from a balance.
        pub fn new(amount: T::Balance, token: Token, type_: AssetType) -> Self {
            NegativeImbalance(amount, token, type_)
        }
    }

    impl<T: Trait> TryDrop for PositiveImbalance<T> {
        fn try_drop(self) -> result::Result<(), Self> {
            self.drop_zero()
        }
    }

    impl<T: Trait> Imbalance<T::Balance> for PositiveImbalance<T> {
        type Opposite = NegativeImbalance<T>;

        fn zero() -> Self {
            PositiveImbalance::new(Zero::zero(), Module::<T>::TOKEN.to_vec(), AssetType::Free)
        }

        fn drop_zero(self) -> result::Result<(), Self> {
            if self.0.is_zero() {
                Ok(())
            } else {
                Err(self)
            }
        }

        fn split(self, amount: T::Balance) -> (Self, Self) {
            let first = self.0.min(amount);
            let second = self.0 - first;
            // create new object pair
            let r = (
                Self(first, self.1.clone(), self.2),
                Self(second, self.1.clone(), self.2),
            );
            // drop self object
            mem::forget(self);
            r
        }

        fn merge(mut self, other: Self) -> Self {
            self.0 = self.0.saturating_add(other.0);
            // drop other object
            mem::forget(other);
            self
        }

        fn subsume(&mut self, other: Self) {
            self.0 = self.0.saturating_add(other.0);
            // drop other object
            mem::forget(other);
        }

        fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
            let (a, b) = (self.0, other.0);
            let r = if a >= b {
                Ok(Self::new(a - b, self.1.clone(), self.2))
            } else {
                Err(NegativeImbalance::new(b - a, self.1.clone(), self.2))
            };
            // drop tuple object
            mem::forget((self, other));
            r
        }

        fn peek(&self) -> T::Balance {
            self.0.clone()
        }
    }

    impl<T: Trait> TryDrop for NegativeImbalance<T> {
        fn try_drop(self) -> result::Result<(), Self> {
            self.drop_zero()
        }
    }

    impl<T: Trait> Imbalance<T::Balance> for NegativeImbalance<T> {
        type Opposite = PositiveImbalance<T>;

        fn zero() -> Self {
            NegativeImbalance::new(Zero::zero(), Module::<T>::TOKEN.to_vec(), AssetType::Free)
        }

        fn drop_zero(self) -> result::Result<(), Self> {
            if self.0.is_zero() {
                Ok(())
            } else {
                Err(self)
            }
        }

        fn split(self, amount: T::Balance) -> (Self, Self) {
            let first = self.0.min(amount);
            let second = self.0 - first;
            // create object pair
            let r = (
                Self(first, self.1.clone(), self.2),
                Self(second, self.1.clone(), self.2),
            );
            // drop self
            mem::forget(self);
            r
        }

        fn merge(mut self, other: Self) -> Self {
            self.0 = self.0.saturating_add(other.0);
            // drop other
            mem::forget(other);
            self
        }

        fn subsume(&mut self, other: Self) {
            self.0 = self.0.saturating_add(other.0);
            // drop other
            mem::forget(other);
        }

        fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
            let (a, b) = (self.0, other.0);
            let r = if a >= b {
                Ok(Self::new(a - b, self.1.clone(), self.2))
            } else {
                Err(PositiveImbalance::new(b - a, self.1.clone(), self.2))
            };
            mem::forget((self, other));
            r
        }

        fn peek(&self) -> T::Balance {
            self.0.clone()
        }
    }

    impl<T: Trait> Drop for PositiveImbalance<T> {
        /// Basic drop handler will just square up the total issuance.
        fn drop(&mut self) {
            TotalAssetBalance::<T>::mutate(&self.1, |map| {
                let balance = map.entry(self.2).or_default();
                *balance = balance.saturating_add(self.0)
            })
        }
    }

    impl<T: Trait> Drop for NegativeImbalance<T> {
        /// Basic drop handler will just square up the total issuance.
        fn drop(&mut self) {
            TotalAssetBalance::<T>::mutate(&self.1, |map| {
                let balance = map.entry(self.2).or_default();
                let new_balance = balance.saturating_sub(self.0);
                if new_balance == Zero::zero() {
                    // remove Zero balance to save space
                    map.remove(&self.2);
                } else {
                    *balance = new_balance;
                }
            })
        }
    }
}
