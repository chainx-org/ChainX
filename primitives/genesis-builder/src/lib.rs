// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! The genesis builder primitives.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub use self::genesis_params::*;

#[cfg(feature = "std")]
mod genesis_params {
    use serde::{Deserialize, Serialize};

    use chainx_primitives::Balance;

    fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<u128>().map_err(serde::de::Error::custom)
    }

    /// Genesis
    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct XbtcInfo {
        pub balance: Balance,
        #[serde(deserialize_with = "deserialize_u128")]
        pub weight: u128,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct XbtcMiner<AccountId> {
        pub who: AccountId,
        #[serde(deserialize_with = "deserialize_u128")]
        pub weight: u128,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct XMiningAssetParams<AccountId> {
        pub xbtc_miners: Vec<XbtcMiner<AccountId>>,
        pub xbtc_info: XbtcInfo,
    }
}
