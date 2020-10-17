// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::HashMap;

use serde::Deserialize;

use chainx_primitives::{AccountId, Balance};

macro_rules! json_from_str {
    ($file:expr) => {
        serde_json::from_str(include_str!($file))
            .map_err(|e| log::error!("{:?}", e))
            .expect("JSON was not well-formatted")
    };
}

fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u128>().map_err(serde::de::Error::custom)
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidatorInfo {
    who: AccountId,
    referral_id: String,
    self_bonded: Balance,
    total_nomination: Balance,
    #[serde(deserialize_with = "deserialize_u128")]
    total_weight: u128,
}

pub fn validators() -> Vec<(AccountId, Vec<u8>, Balance, Balance, u128)> {
    let validators: Vec<ValidatorInfo> = json_from_str!("./res/genesis_validators.json");
    validators
        .into_iter()
        .map(|v| {
            (
                v.who,
                v.referral_id.as_bytes().to_vec(),
                v.self_bonded,
                v.total_nomination,
                v.total_weight,
            )
        })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Nomination {
    nominee: AccountId,
    nomination: Balance,
    #[serde(deserialize_with = "deserialize_u128")]
    weight: u128,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NominatorInfo {
    nominator: AccountId,
    nominations: Vec<Nomination>,
}

pub fn nominators() -> Vec<(AccountId, Vec<(AccountId, Balance, u128)>)> {
    let nominators: Vec<NominatorInfo> = json_from_str!("./res/genesis_nominators.json");
    nominators
        .into_iter()
        .map(|n| {
            (
                n.nominator,
                n.nominations
                    .into_iter()
                    .map(|nom| (nom.nominee, nom.nomination, nom.weight))
                    .collect(),
            )
        })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct XbtcInfo {
    balance: Balance,
    #[serde(deserialize_with = "deserialize_u128")]
    weight: u128,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct XbtcMiner {
    who: AccountId,
    #[serde(deserialize_with = "deserialize_u128")]
    weight: u128,
}

#[derive(Debug, serde::Deserialize)]
struct XMiningAssetParams {
    xbtc_miners: Vec<XbtcMiner>,
    xbtc_info: XbtcInfo,
}

pub fn xmining_asset() -> (Vec<(AccountId, u128)>, u128) {
    let params: XMiningAssetParams = json_from_str!("./res/genesis_xminingasset.json");
    let XMiningAssetParams {
        xbtc_miners,
        xbtc_info,
    } = params;
    (
        xbtc_miners.into_iter().map(|m| (m.who, m.weight)).collect(),
        xbtc_info.weight,
    )
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpecialAccounts {
    council: AccountId,
    team: AccountId,
    pots: HashMap<AccountId, AccountId>,
}

pub fn special_accounts() -> (AccountId, AccountId, Vec<(AccountId, AccountId)>) {
    let special_accounts: SpecialAccounts = json_from_str!("./res/genesis_special_accounts.json");
    (
        special_accounts.council,
        special_accounts.team,
        special_accounts.pots.into_iter().collect(),
    )
}
