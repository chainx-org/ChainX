// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::convert::TryFrom;

use hex_literal::hex;
use sp_core::sr25519;

use crate::chain_spec::get_account_id_from_seed;
use chainx_primitives::AccountId;
use chainx_runtime::{trustees, Chain, TrusteeInfoConfig};

// (account_id, about, hot_key, cold_key)
pub type TrusteeParams = (AccountId, Vec<u8>, Vec<u8>, Vec<u8>);
macro_rules! btc_trustee_key {
    ($btc_pubkey:expr) => {{
        trustees::bitcoin::BtcTrusteeType::try_from(
            hex::decode($btc_pubkey).expect("hex decode failed"),
        )
        .expect("btc trustee generation failed")
        .into()
    }};
}
fn btc_trustee_gen(seed: &str, hot_pubkey: &str, cold_pubkey: &str) -> TrusteeParams {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        seed.as_bytes().to_vec(),      // About
        btc_trustee_key!(hot_pubkey),  // Hot key
        btc_trustee_key!(cold_pubkey), // Cold key
    )
}

pub fn local_testnet_trustees() -> Vec<(Chain, TrusteeInfoConfig, Vec<TrusteeParams>)> {
    let btc_trustees = vec![
        btc_trustee_gen(
            "Alice",
            "0376b9649206c74cc3dad6332c3a86d925a251bf9a55e6381f5d67b29a47559634", // hot key
            "0300849497d4f88ebc3e1bc2583677c5abdbd3b63640b3c5c50cd4628a33a2a2ca", // colde key
        ),
        btc_trustee_gen(
            "Bob",
            "0285eed6fa121c3a82ba6d0c37fa37e72bb06740761bfe9f294d2fa95fe237d5ba",
            "032122032ae9656f9a133405ffe02101469a8d62002270a33ceccf0e40dda54d08",
        ),
        btc_trustee_gen(
            "Charlie",
            "036e1b175cc285b62a8b86e4ea94f32d627b36d60673b37eb3dd07d7b8c9ae6ddb",
            "02b3cc747f572d33f12870fa6866aebbfd2b992ba606b8dc89b676b3697590ad63",
        ),
    ];

    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };

    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}

#[cfg(feature = "runtime-benchmarks")]
pub fn benchmarks_trustees() -> Vec<(Chain, TrusteeInfoConfig, Vec<TrusteeParams>)> {
    let btc_trustees = vec![
        // 1
        btc_trustee_gen(
            "Alice",
            "02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6",
            "0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88",
        ),
        // 2
        btc_trustee_gen(
            "Bob",
            "0244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d",
            "02e4631e46255571122d6e11cda75d5d601d5eb2585e65e4e87fe9f68c7838a278",
        ),
        // 3
        btc_trustee_gen(
            "Charlie",
            "03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102", // hot key
            "0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3", // colde key
        ),
    ];

    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };

    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}

pub fn staging_testnet_trustees() -> Vec<(Chain, TrusteeInfoConfig, Vec<TrusteeParams>)> {
    let btc_trustees = vec![
        (
            // 5Ca46gRUa2oS6GukzKph8qFfn4WdhP5yhuRaTuzaXsKjfGgM
            hex!["16624186f2ea93a21f34e00ae622959e40d841231b26e625be93f75137b2a10d"].into(),
            b"Validator1".to_vec(), // About
            btc_trustee_key!("0376b9649206c74cc3dad6332c3a86d925a251bf9a55e6381f5d67b29a47559634"), // Hot key
            btc_trustee_key!("0300849497d4f88ebc3e1bc2583677c5abdbd3b63640b3c5c50cd4628a33a2a2ca"), // Cold key
        ),
        (
            // 5DV17DNeRCidmacaP1MdhD8YV8A94PmVyr4eRcKq8tG6Q17C
            hex!["3ec431c8b3ae28095ad652f5531a770ef21e59779d4a3a46e0217baa4c614624"].into(),
            b"Validator2".to_vec(), // About
            btc_trustee_key!("0285eed6fa121c3a82ba6d0c37fa37e72bb06740761bfe9f294d2fa95fe237d5ba"), // Hot key
            btc_trustee_key!("032122032ae9656f9a133405ffe02101469a8d62002270a33ceccf0e40dda54d08"), // Cold key
        ),
        (
            // 5ERY5k4cDMhhE7B8PRA26fCs1VbHNZJAhHoiuZhzP18cxq8T
            hex!["685bb75b531394c4d522003784cc62fa15fcab8fe16c19c3f4a1eeae308afa4f"].into(),
            b"Validator3".to_vec(), // About
            btc_trustee_key!("036e1b175cc285b62a8b86e4ea94f32d627b36d60673b37eb3dd07d7b8c9ae6ddb"), // Hot key
            btc_trustee_key!("02b3cc747f572d33f12870fa6866aebbfd2b992ba606b8dc89b676b3697590ad63"), // Cold key
        ),
    ];

    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };

    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}
