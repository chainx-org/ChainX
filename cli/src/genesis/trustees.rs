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
            "035b8fb240f808f4d3d0d024fdf3b185b942e984bba81b6812b8610f66d59f3a84", // hot key
            "0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3", // colde key
        ),
        btc_trustee_gen(
            "Bob",
            "02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee",
            "020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f",
        ),
        btc_trustee_gen(
            "Charlie",
            "0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd",
            "02a83c80e371ddf0a29006096765d060190bb607ec015ba6023b40ace582e13b99",
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

// TODO wait for pubkey
pub fn testnet_trustees() -> Vec<(Chain, TrusteeInfoConfig, Vec<TrusteeParams>)> {
    let btc_trustees = vec![
        (
            hex!["9ae581fef1fc06828723715731adcf810e42ce4dadad629b1b7fa5c3c144a81d"].into(),
            b"".to_vec(), // About
            btc_trustee_key!("035b8fb240f808f4d3d0d024fdf3b185b942e984bba81b6812b8610f66d59f3a84"), // Hot key
            btc_trustee_key!("0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3"), // Cold key
        ),
        (
            hex!["9ae581fef1fc06828723715731adcf810e42ce4dadad629b1b7fa5c3c144a81d"].into(),
            b"".to_vec(), // About
            btc_trustee_key!("02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee"), // Hot key
            btc_trustee_key!("020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f"), // Cold key
        ),
        (
            hex!["9ae581fef1fc06828723715731adcf810e42ce4dadad629b1b7fa5c3c144a81d"].into(),
            b"".to_vec(), // About
            btc_trustee_key!("0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd"), // Hot key
            btc_trustee_key!("02a83c80e371ddf0a29006096765d060190bb607ec015ba6023b40ace582e13b99"), // Cold key
        ),
    ];

    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };

    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}
