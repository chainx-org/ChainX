// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use std::convert::TryFrom;

use hex_literal::hex;
use serde::Deserialize;

use sp_core::sr25519;

use chainx_primitives::AccountId;
//  todo! replace:
// use chain_runtime::{
//     h256_rev, trustees, BtcHeader, BtcNetwork, Chain, Compact as BtcCompact, TrusteeInfoConfig,
//     H256 as BtcHash,
// };
use dev_runtime::{
    h256_rev, trustees, BtcHeader, BtcNetwork, Chain, Compact as BtcCompact, TrusteeInfoConfig,
    H256 as BtcHash,
};

use crate::chain_spec::get_account_id_from_seed;

#[derive(Debug, Deserialize)]
pub struct BtcGenesisParams {
    pub network: BtcNetwork,
    pub confirmation_number: u32,
    pub height: u32,
    hash: String,
    version: u32,
    previous_header_hash: String,
    merkle_root_hash: String,
    time: u32,
    bits: BtcCompact,
    nonce: u32,
}

impl BtcGenesisParams {
    /// Return the block hash.
    ///
    /// Indicating user-visible serializations of this hash should be backward.
    pub fn hash(&self) -> BtcHash {
        h256_rev(&self.hash)
    }

    /// Return the block header.
    ///
    /// Indicating user-visible serializations of `previous_header_hash` and `merkle_root_hash`
    /// should be backward.
    pub fn header(&self) -> BtcHeader {
        BtcHeader {
            version: self.version,
            previous_header_hash: h256_rev(&self.previous_header_hash),
            merkle_root_hash: h256_rev(&self.merkle_root_hash),
            time: self.time,
            bits: self.bits,
            nonce: self.nonce,
        }
    }
}

pub fn btc_genesis_params(res: &str) -> BtcGenesisParams {
    let params: BtcGenesisParams = serde_json::from_str(res).expect("JSON was not well-formatted");
    assert_eq!(params.header().hash(), params.hash());
    params
}

// (account_id, about, hot_key, cold_key)
pub type BtcTrusteeParams = (AccountId, Vec<u8>, Vec<u8>, Vec<u8>);

macro_rules! btc_trustee_key {
    ($btc_pubkey:expr) => {{
        trustees::bitcoin::BtcTrusteeType::try_from(
            hex::decode($btc_pubkey).expect("hex decode failed"),
        )
        .expect("btc trustee generation failed")
        .into()
    }};
}

fn btc_trustee_gen(seed: &str, hot_pubkey: &str, cold_pubkey: &str) -> BtcTrusteeParams {
    (
        get_account_id_from_seed::<sr25519::Public>(seed), // Account Id
        seed.as_bytes().to_vec(),                          // Seed Bytes.
        btc_trustee_key!(hot_pubkey),                      // Hot Key
        btc_trustee_key!(cold_pubkey),                     // Cold Key
    )
}

pub fn local_testnet_trustees() -> Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)> {
    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };

    let btc_trustees = vec![
        btc_trustee_gen(
            "Alice",
            "0283f579dd2380bd31355d066086e1b4d46b518987c1f8a64d4c0101560280eae2",
            "0300849497d4f88ebc3e1bc2583677c5abdbd3b63640b3c5c50cd4628a33a2a2ca",
        ),
        btc_trustee_gen(
            "Bob",
            "027a0868a14bd18e2e45ff3ad960f892df8d0edd1a5685f0a1dc63c7986d4ad55d",
            "032122032ae9656f9a133405ffe02101469a8d62002270a33ceccf0e40dda54d08",
        ),
        btc_trustee_gen(
            "Charlie",
            "02c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f",
            "02b3cc747f572d33f12870fa6866aebbfd2b992ba606b8dc89b676b3697590ad63",
        ),
    ];

    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}

#[cfg(feature = "runtime-benchmarks")]
pub fn benchmarks_trustees() -> Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)> {
    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };

    let btc_trustees = vec![
        // 1
        btc_trustee_gen(
            "Alice",
            "0283f579dd2380bd31355d066086e1b4d46b518987c1f8a64d4c0101560280eae2",
            "0300849497d4f88ebc3e1bc2583677c5abdbd3b63640b3c5c50cd4628a33a2a2ca",
        ),
        // 2
        btc_trustee_gen(
            "Bob",
            "027a0868a14bd18e2e45ff3ad960f892df8d0edd1a5685f0a1dc63c7986d4ad55d",
            "032122032ae9656f9a133405ffe02101469a8d62002270a33ceccf0e40dda54d08",
        ),
        // 3
        btc_trustee_gen(
            "Charlie",
            "02c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f",
            "02b3cc747f572d33f12870fa6866aebbfd2b992ba606b8dc89b676b3697590ad63",
        ),
    ];

    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}

#[allow(dead_code)]
pub fn mainnet_trustees() -> Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)> {
    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };

    let btc_trustees = vec![
        (
            // 1. 5SY3yajabLKYcuxPjXwLMc7p6WDC4Tv1H3sVMx6Hmjtxycji
            hex!["80269f1c8712f25eb590fc849b89c79cc9b2309b2b2696e96d5610a08581b8aa"].into(),
            b"MathWallet".to_vec(),
            btc_trustee_key!("02e2b2720a9e54617ba87fca287c3d7f9124154d30fa8dc9cd260b6b254e1d7aea"),
            btc_trustee_key!("03615bee4a2f2e80605be8730dc9630b002ad83b068a902df03b155797357030f7"),
        ),
        (
            // 2. 5V7ygyZ53psrNSFgT3n7Xnxd6r7eC6bga3eA4W8KYEs75ZeC
            hex!["f27f00cdfefa8c98d0f3fbf8835fbf7f269a74963d87a56942323537f533715e"].into(),
            b"GfNetwork".to_vec(),
            btc_trustee_key!("0219fc860933a1362bc5e0a0bbe1b33a47aedf904765f4a85cd166ba1d767927ee"),
            btc_trustee_key!("037f8d0b44a282a89352b238b2d09f996df290aa65e0a95e6c99a445072ce390ce"),
        ),
        (
            // 3. 5ReDj2o2xRQowpcrRdrCq3hR4cj1dJgj239dGMHnB9QzAnPa
            hex!["589eadd1fec281804b6f080d89b4262114e1e7485962a27259865901b615b895"].into(),
            b"CybexDex".to_vec(),
            btc_trustee_key!("02b921cb319a14c6887b12cee457453f720e88808a735a578d6c57aba0c74e5af3"),
            btc_trustee_key!("0281687791324c3d99d9bd39370baf4c138b1e1670a9939a406e3ac22577e39c00"),
        ),
        (
            // 4. 5RjfjwXjzJtVd6EiTCG3RsJmUM9h4FgocswJyAaLvuBicwE4
            hex!["5cc67b8a790f517a0fd02683be59e8e461cc6e72749d26f46a65fce24d18e1a4"].into(),
            b"PolkaX".to_vec(),
            btc_trustee_key!("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6"),
            btc_trustee_key!("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88"),
        ),
        (
            // 5. 5QpTfTDYSLWkuVEvRqEcugQtFZnhE3qyJLCzwGQgdzNRpiSQ
            hex!["3431d1ef4f8b1b75136ce26ea9e6e0379c68d77abfa37cc5303bcb0d5ffe0c64"].into(),
            b"Wetez".to_vec(),
            btc_trustee_key!("0346aa7ade0b567b34182cacf9444deb44ee829e14705dc87175107dd09d5dbf40"),
            btc_trustee_key!("0299b5c30667f2e80ddccbac8d112e52387fa1056ef2510c0b7a627215eb0a4550"),
        ),
        (
            // 6. 5Pjajd12o9hVixBPRPHZEdjsrct3NZp9Ge7QP4PiSivQrBZa
            hex!["043d2a66efc5646eaea2e3b0235b8d577538aa001148b16988997fe02916f06a"].into(),
            b"buildlinks".to_vec(),
            btc_trustee_key!("034d3e7f87e69c6c71df6052b44f9ed99a3d811613140ebf09f8fdaf904a2e1de8"),
            btc_trustee_key!("02c179b0e69b342bf295200fa072bd2a4e956a2b74d7319c256946bc349c67d209"),
        ),
    ];

    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}
