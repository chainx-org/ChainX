// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

/// Balance type when interacting with RPC.
pub type RpcBalance<Balance> = U128<Balance>;

/// Price type of order when interacting with RPC.
pub type RpcPrice<Price> = U128<Price>;

/// WeightType when interacting with RPC.
pub type RpcWeightType = U128<u128>;

/// This struct provides a wrapper of Balance in runtime due to the u128 serde issue.
///
/// # Example
///
/// ```no_compile
/// use xpallet_support::RpcBalance;
///
/// sp_api::decl_runtime_apis! {
///     pub trait PalletApi<Balance> where
///         Balance: Codec,
///     {
///         /// Get total asset balance.
///         ///
///         /// Ideally:
///         ///     fn asset_balance() -> Balance;
///         ///
///         /// Workaround for the u128 serde issue:
///         ///     fn asset_balance() -> RpcBalance<Balance>;
///         ///
///         /// What you need to do is to replace Balance with RpcBalance<Balance>
///         /// in returned value when interacting with RPC API.
///         /// For the other u128 cases, just U128<Balance> directly.
///         fn total_asset_balance() -> RpcBalance<Balance>;
///     }
/// }
/// ```
///
/// TODO: remove this workaround once https://github.com/paritytech/substrate/issues/4641 is resolved.
#[derive(Eq, PartialEq, Clone, codec::Encode, codec::Decode, Default)]
#[cfg_attr(feature = "std", derive(std::fmt::Debug))]
pub struct U128<Balance>(Balance);

impl<Balance> From<Balance> for U128<Balance> {
    fn from(inner: Balance) -> Self {
        Self(inner)
    }
}

#[cfg(feature = "std")]
impl<Balance: std::fmt::Display> std::fmt::Display for U128<Balance> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl<Balance: std::str::FromStr> std::str::FromStr for U128<Balance> {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s
            .parse::<Balance>()
            .map_err(|_| "Parse Balance from String failed")?;
        Ok(Self(inner))
    }
}

#[cfg(feature = "std")]
impl<Balance: std::string::ToString> serde::ser::Serialize for U128<Balance> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.0.to_string().serialize(serializer)
    }
}

#[cfg(feature = "std")]
impl<'de, Balance: std::str::FromStr> serde::de::Deserialize<'de> for U128<Balance> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}
