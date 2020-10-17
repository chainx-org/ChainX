// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::HashMap;

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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidatorInfo {
    who: AccountId,
    referral_id: String,
    self_bonded: Balance,
    total_nomination: Balance,
    total_weight: String,
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
                as_u128(&v.total_weight),
            )
        })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Nomination {
    nominee: AccountId,
    nomination: Balance,
    weight: String,
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
                    .map(|nom| (nom.nominee, nom.nomination, as_u128(&nom.weight)))
                    .collect(),
            )
        })
        .collect()
}

fn as_u128(s: &str) -> u128 {
    s.parse::<u128>().expect("parse u128 from string failed")
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct XbtcInfo {
    balance: Balance,
    weight: String,
}

pub fn xbtc_weight() -> u128 {
    let xbtc_info: XbtcInfo = json_from_str!("./res/genesis_xbtc_info.json");
    as_u128(&xbtc_info.weight)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct XbtcMiner {
    who: AccountId,
    weight: String,
}

pub fn xbtc_miners() -> Vec<(AccountId, u128)> {
    let xbtc_miners: Vec<XbtcMiner> = json_from_str!("./res/genesis_xbtc_miners.json");
    xbtc_miners
        .into_iter()
        .map(|m| (m.who, as_u128(&m.weight)))
        .collect()
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
