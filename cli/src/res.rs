// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::HashMap;

use xp_genesis_builder::{WellknownAccounts, XMiningAssetParams, XStakingParams};

use chainx_primitives::{AccountId, Balance};

macro_rules! json_from_str {
    ($file:expr) => {
        serde_json::from_str(include_str!($file))
            .map_err(|e| log::error!("{:?}", e))
            .expect("JSON was not well-formatted")
    };
}

#[derive(Debug, serde::Deserialize)]
struct BalanceInfo {
    who: AccountId,
    free: Balance,
}

pub fn balances() -> Vec<(AccountId, Balance)> {
    let balances: Vec<BalanceInfo> = json_from_str!("./res/genesis_balances.json");
    balances.into_iter().map(|b| (b.who, b.free)).collect()
}

pub fn xassets() -> Vec<(AccountId, Balance)> {
    let balances: Vec<BalanceInfo> = json_from_str!("./res/genesis_xassets.json");
    balances.into_iter().map(|b| (b.who, b.free)).collect()
}

pub fn xstaking() -> XStakingParams<AccountId, Balance> {
    json_from_str!("./res/genesis_xstaking.json")
}

pub fn xmining_asset() -> XMiningAssetParams<AccountId> {
    json_from_str!("./res/genesis_xminingasset.json")
}

#[derive(Debug, serde::Deserialize)]
pub struct ConcreteAccounts {
    council: AccountId,
    team: AccountId,
    pots: HashMap<AccountId, AccountId>,
}

pub fn wellknown_accounts() -> WellknownAccounts<AccountId> {
    let accounts: ConcreteAccounts = json_from_str!("./res/genesis_special_accounts.json");
    WellknownAccounts {
        legacy_council: accounts.council,
        legacy_team: accounts.team,
        legacy_pots: accounts.pots.into_iter().collect(),
    }
}
