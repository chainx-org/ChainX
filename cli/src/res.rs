// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

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

// testnet
pub fn testnet_btc_genesis_header() -> BitcoinParams {
    let raw: BitcoinGenesisHeader =
        serde_json::from_str(include_str!("./res/btc_genesis_header_testnet.json"))
            .expect("JSON was not well-formatted");
    build_bitcoin_params(raw, 6u32)
}

// mainnet
pub fn mainnet_btc_genesis_header() -> BitcoinParams {
    let raw: BitcoinGenesisHeader =
        serde_json::from_str(include_str!("./res/btc_genesis_header_mainnet.json"))
            .expect("JSON was not well-formatted");
    build_bitcoin_params(raw, 4u32)
}
