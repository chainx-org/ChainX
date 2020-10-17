// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use xp_genesis_builder::{BalancesParams, FreeBalanceInfo, XMiningAssetParams, XStakingParams};

use chainx_primitives::{AccountId, Balance};

macro_rules! json_from_str {
    ($file:expr) => {
        serde_json::from_str(include_str!($file))
            .map_err(|e| log::error!("{:?}", e))
            .expect("JSON was not well-formatted")
    };
}

pub fn balances() -> BalancesParams<AccountId, Balance> {
    json_from_str!("./res/genesis_balances.json")
}

pub fn xassets() -> Vec<FreeBalanceInfo<AccountId, Balance>> {
    json_from_str!("./res/genesis_xassets.json")
}

pub fn xstaking() -> XStakingParams<AccountId, Balance> {
    json_from_str!("./res/genesis_xstaking.json")
}

pub fn xmining_asset() -> XMiningAssetParams<AccountId> {
    json_from_str!("./res/genesis_xminingasset.json")
}
