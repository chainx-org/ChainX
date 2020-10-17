// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! The genesis builder primitives.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub use self::genesis_params::*;

#[cfg(feature = "std")]
mod genesis_params {
    use chainx_primitives::Balance;
    use serde::{Deserialize, Serialize};

    fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<u128>().map_err(serde::de::Error::custom)
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ValidatorInfo<AccountId, Balance> {
        pub who: AccountId,
        #[serde(with = "xpallet_support::serde_text")]
        pub referral_id: Vec<u8>,
        pub self_bonded: Balance,
        pub total_nomination: Balance,
        #[serde(deserialize_with = "deserialize_u128")]
        pub total_weight: u128,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct Nomination<AccountId, Balance> {
        pub nominee: AccountId,
        pub nomination: Balance,
        #[serde(deserialize_with = "deserialize_u128")]
        pub weight: u128,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct NominatorInfo<AccountId, Balance> {
        pub nominator: AccountId,
        pub nominations: Vec<Nomination<AccountId, Balance>>,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct XStakingParams<AccountId, Balance> {
        pub validators: Vec<ValidatorInfo<AccountId, Balance>>,
        pub nominators: Vec<NominatorInfo<AccountId, Balance>>,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct WellknownAccounts<AccountId> {
        pub legacy_council: AccountId,
        pub legacy_team: AccountId,
        pub legacy_pots: Vec<(AccountId, AccountId)>,
    }

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
