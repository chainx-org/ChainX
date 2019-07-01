// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use rstd::{prelude::*, result, slice::Iter};
use support::dispatch::Result;
use support::traits::{Imbalance, SignedImbalance};
use support::StorageMap;

use primitives::traits::{Saturating, Zero};
// ChainX
pub use xr_primitives::{Desc, Memo, Token};

use super::traits::ChainT;
use super::{Module, Trait};

pub use self::imbalances::{NegativeImbalance, PositiveImbalance};

const MAX_TOKEN_LEN: usize = 32;
const MAX_DESC_LEN: usize = 128;

pub type TokenString = &'static [u8];
pub type DescString = TokenString;
pub type Precision = u16;

pub type SignedImbalanceT<T> = SignedImbalance<<T as Trait>::Balance, PositiveImbalance<T>>;

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
    #[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
    Chain {
        ChainX,
        Bitcoin,
        Ethereum,
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

#[cfg(feature = "std")]
impl std::fmt::Debug for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
}

impl Asset {
    pub fn new(
        token: Token,
        token_name: Token,
        chain: Chain,
        precision: Precision,
        desc: Desc,
    ) -> result::Result<Self, &'static str> {
        let a = Asset {
            token,
            token_name,
            chain,
            precision,
            desc,
        };
        a.is_valid()?;
        Ok(a)
    }
    pub fn is_valid(&self) -> Result {
        is_valid_token(&self.token)?;
        is_valid_token_name(&self.token_name)?;
        is_valid_desc(&self.desc)
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
    pub fn set_desc(&mut self, desc: Desc) {
        self.desc = desc
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
}

define_enum!(
    #[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
    AssetType {
        Free,
        ReservedStaking,
        ReservedStakingRevocation,
        ReservedWithdrawal,
        ReservedDexSpot,
        ReservedDexFuture,
        ReservedCurrency,
    }
);

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Free
    }
}

define_enum!(
    #[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
    AssetLimit {
        CanMove,
        CanTransfer,
        CanDestroyWithdrawal,
        CanDestroyFree,
    }
);

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum AssetErr {
    NotEnough,
    OverFlow,
    TotalAssetNotEnough,
    TotalAssetOverFlow,
    InvalidToken,
    InvalidAccount,
    NotAllow,
}

impl AssetErr {
    pub fn info(self) -> &'static str {
        match self {
            AssetErr::NotEnough => "balance too low for this account",
            AssetErr::OverFlow => "balance too high for this account",
            AssetErr::TotalAssetNotEnough => "total balance too low for this asset",
            AssetErr::TotalAssetOverFlow => "total balance too high for this asset",
            AssetErr::InvalidToken => "not a valid token for this account",
            AssetErr::InvalidAccount => "account Locked",
            AssetErr::NotAllow => "not allow to do",
        }
    }
}

/// Token can only use ASCII alphanumeric character or "-.|~".
pub fn is_valid_token(v: &[u8]) -> Result {
    if v.len() > MAX_TOKEN_LEN || v.is_empty() {
        return Err("Token length is zero or too long.");
    }
    let is_valid = |c: &u8| -> bool { c.is_ascii_alphanumeric() || "-.|~".as_bytes().contains(c) };
    for c in v.iter() {
        if !is_valid(c) {
            return Err("Token can only use ASCII alphanumeric character or '-', '.', '|', '~'.");
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
pub fn is_valid_token_name(name: &[u8]) -> Result {
    if name.len() > MAX_TOKEN_LEN || name.is_empty() {
        return Err("Token name is zero or too long.");
    }
    xaccounts::is_xss_proof(name)?;
    for c in name.iter() {
        if is_ascii_invisible(c) {
            return Err("Token name can not use an invisible ASCII char.");
        }
    }
    Ok(())
}

/// A valid desc should be visible ASCII chars only and not too long.
pub fn is_valid_desc(desc: &[u8]) -> Result {
    if desc.len() > MAX_DESC_LEN {
        return Err("Token desc too long");
    }
    xaccounts::is_xss_proof(desc)?;
    for c in desc.iter() {
        if is_ascii_invisible(c) {
            return Err("Desc can not use an invisiable ASCII char.");
        }
    }
    Ok(())
}

/// A valid memo should have a legal length and be xss proof.
pub fn is_valid_memo<T: Trait>(memo: &Memo) -> Result {
    if memo.len() as u32 > Module::<T>::memo_len() {
        return Err("memo is too long");
    }
    xaccounts::is_xss_proof(memo)
}

mod imbalances {
    use super::{result, AssetType, ChainT, Imbalance, Saturating, StorageMap, Token, Zero};
    use crate::{Module, TotalAssetBalance, Trait};
    use rstd::mem;

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
                *balance = balance.saturating_sub(self.0)
            })
        }
    }

}
