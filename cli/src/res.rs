// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use chainx_primitives::{AccountId, Balance, BlockNumber};
use chainx_runtime::{h256_conv_endian_from_str, BtcCompact, BtcHeader, BtcNetwork};

#[derive(Debug, serde::Deserialize)]
struct BitcoinGenesisHeader {
    version: u32,
    previous_header_hash: String,
    merkle_root_hash: String,
    time: u32,
    bits: u32,
    nonce: u32,
    height: u32,
    hash: String,
    network_id: String,
}

fn as_btc_network(network_id: &str) -> BtcNetwork {
    match network_id {
        "Mainnet" => BtcNetwork::Mainnet,
        "Testnet" => BtcNetwork::Testnet,
        _ => unreachable!("network_id is either Mainnet or Testnet"),
    }
}

pub struct BitcoinParams {
    pub genesis_info: (BtcHeader, u32),
    pub genesis_hash: xpallet_gateway_bitcoin::H256,
    pub network: BtcNetwork,
    pub confirmed_count: u32,
}

fn build_bitcoin_params(raw: BitcoinGenesisHeader, confirmed_count: u32) -> BitcoinParams {
    let as_h256 = |s: &str| h256_conv_endian_from_str(s);
    BitcoinParams {
        genesis_info: (
            BtcHeader {
                version: raw.version,
                previous_header_hash: as_h256(&raw.previous_header_hash),
                merkle_root_hash: as_h256(&raw.merkle_root_hash),
                time: raw.time,
                bits: BtcCompact::new(raw.bits),
                nonce: raw.nonce,
            },
            raw.height,
        ),
        genesis_hash: as_h256(&raw.hash),
        network: as_btc_network(&raw.network_id),
        confirmed_count,
    }
}

macro_rules! json_from_str {
    ($file:expr) => {
        serde_json::from_str(include_str!($file))
            .map_err(|e| log::error!("{:?}", e))
            .expect("JSON was not well-formatted")
    };
}

// testnet
pub fn testnet_btc_genesis_header() -> BitcoinParams {
    let raw: BitcoinGenesisHeader = json_from_str!("./res/btc_genesis_header_testnet.json");
    build_bitcoin_params(raw, 6u32)
}

// mainnet
pub fn mainnet_btc_genesis_header() -> BitcoinParams {
    let raw: BitcoinGenesisHeader = json_from_str!("./res/btc_genesis_header_mainnet.json");
    build_bitcoin_params(raw, 4u32)
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
                v.total_weight
                    .parse::<u128>()
                    .expect("Parse u128 from string failed"),
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
                    .map(|nom| {
                        (
                            nom.nominee,
                            nom.nomination,
                            nom.weight
                                .parse::<u128>()
                                .expect("Parse u128 from string failed"),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Unbond {
    target: AccountId,
    unbonded_chunks: Vec<xpallet_mining_staking::Unbonded<Balance, BlockNumber>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnbondInfo {
    nominator: AccountId,
    unbonds: Vec<Unbond>,
}

pub fn unbonds() -> Vec<(AccountId, Vec<(AccountId, Vec<(Balance, BlockNumber)>)>)> {
    let unbonds: Vec<UnbondInfo> = json_from_str!("./res/genesis_unbonds.json");
    unbonds
        .into_iter()
        .map(|info| {
            (
                info.nominator,
                info.unbonds
                    .into_iter()
                    .map(|unbond| {
                        (
                            unbond.target,
                            unbond
                                .unbonded_chunks
                                .into_iter()
                                .map(|i| (i.value, i.locked_until))
                                .collect(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}
