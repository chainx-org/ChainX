// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! The genesis builder primitives.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub use self::genesis_params::*;

#[cfg(feature = "std")]
mod genesis_params {
    use chainx_primitives::Balance;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct AllParams<AccountId, TBalance, AssetBalanceOf, StakingBalanceOf> {
        pub balances: BalancesParams<AccountId, TBalance>,
        pub xassets: Vec<FreeBalanceInfo<AccountId, AssetBalanceOf>>,
        pub xstaking: XStakingParams<AccountId, StakingBalanceOf>,
        pub xmining_asset: XMiningAssetParams<AccountId>,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct FreeBalanceInfo<AccountId, Balance> {
        pub who: AccountId,
        pub free: Balance,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct WellknownAccounts<AccountId> {
        pub legacy_council: AccountId,
        pub legacy_team: AccountId,
        pub legacy_pots: Vec<(AccountId, AccountId)>,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct BalancesParams<AccountId, Balance> {
        pub free_balances: Vec<FreeBalanceInfo<AccountId, Balance>>,
        pub wellknown_accounts: WellknownAccounts<AccountId>,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct ValidatorInfo<AccountId, Balance> {
        pub who: AccountId,
        #[serde(with = "xpallet_support::serde_text")]
        pub referral_id: Vec<u8>,
        pub self_bonded: Balance,
        pub total_nomination: Balance,
        #[serde(with = "xpallet_support::serde_num_str")]
        pub total_weight: u128,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct Nomination<AccountId, Balance> {
        pub nominee: AccountId,
        pub nomination: Balance,
        #[serde(with = "xpallet_support::serde_num_str")]
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
    pub struct XBtcInfo {
        pub balance: Balance,
        #[serde(with = "xpallet_support::serde_num_str")]
        pub weight: u128,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct XBtcMiner<AccountId> {
        pub who: AccountId,
        #[serde(with = "xpallet_support::serde_num_str")]
        pub weight: u128,
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct XMiningAssetParams<AccountId> {
        pub xbtc_miners: Vec<XBtcMiner<AccountId>>,
        pub xbtc_info: XBtcInfo,
    }
}
