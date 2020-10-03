// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use chainx_primitives::{AccountId, Balance};
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
        serde_json::from_str(include_str!($file)).expect("JSON was not well-formatted")
    };
}

// testnet
pub fn testnet_btc_genesis_header_info() -> BitcoinParams {
    let raw: BitcoinGenesisHeader = json_from_str!("./res/btc_genesis_header_testnet.json");
    build_bitcoin_params(raw, 6u32)
}

// mainnet
pub fn mainnet_btc_genesis_header_info() -> BitcoinParams {
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
