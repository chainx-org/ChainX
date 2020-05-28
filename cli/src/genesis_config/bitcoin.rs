// Copyright 2018-2020 Chainpool.

use super::*;
use btc_chain::BlockHeader;
use btc_primitives::{h256_from_rev_str, Compact, H256};

/// Tuple of (BlockHeader, BlockHeight).
type BlockHeaderPair = (BlockHeader, u32);
type BitcoinGenesisInfo = (BlockHeaderPair, H256, u32);

pub fn create_asset() -> Asset {
    Asset::new(
        <xbitcoin::Module<Runtime> as ChainT>::TOKEN.to_vec(), // token
        b"X-BTC".to_vec(),
        Chain::Bitcoin,
        8, // bitcoin precision
        b"ChainX's Cross-chain Bitcoin".to_vec(),
    )
    .unwrap()
}

pub fn testnet_taoism() -> BitcoinGenesisInfo {
    (
        (
            BlockHeader {
                version: 536870912,
                previous_header_hash: h256_from_rev_str(
                    "00000000000b494dc8ec94e46e2d111c6b9a317e7300494544a147e15371ff58",
                ),
                merkle_root_hash: h256_from_rev_str(
                    "670cd88ba0dd51650a444c744b8088653dba381bf091366ecc41dba0e1b483ff",
                ),
                time: 1573628044,
                bits: Compact::new(436469756),
                nonce: 3891368516,
            },
            1608246,
        ),
        h256_from_rev_str("00000000000000927abc8c28ddd2c0ee46cc47dadb4c45ee14ff2a0307e1b896"),
        1,
    )
}

// bitcoin testnet
pub fn testnet_mohism() -> BitcoinGenesisInfo {
    (
        (
            BlockHeader {
                version: 0x20400000,
                previous_header_hash: h256_from_rev_str(
                    "00000000747224aab97a80577eb3fefcc6e182ccb916a2d9f16b3cdee6ac46bc",
                ),
                merkle_root_hash: h256_from_rev_str(
                    "e3b332dfe87440c2e9c106fa32de1eb63adde90748a7f6e9eff7c23e09926690",
                ),
                time: 1589721767,
                bits: Compact::new(0x1a0ffff0),
                nonce: 0x6ba03668,
            },
            1745290,
        ),
        h256_from_rev_str("0000000000000afef24ac300f11b64115335471fa46dd8f8a8b4f9fe575ad38b"),
        1,
    )
}

// bitcoin mainnet for confucianism
pub fn mainnet_confucianism() -> BitcoinGenesisInfo {
    (
        (
            BlockHeader {
                version: 0x27ffe000,
                previous_header_hash: h256_from_rev_str(
                    "0000000000000000000e87ecbff47d9ab75e78d92328d5951351f9702597dace",
                ),
                merkle_root_hash: h256_from_rev_str(
                    "783ffb1dd4004232a041ad7d1cb3d3dbc1583b9f27ad558d63db873e880383f6",
                ),
                time: 1589940416,
                bits: Compact::new(0x171297f6),
                nonce: 0x0dc797f9,
            },
            631_008,
        ),
        h256_from_rev_str("0000000000000000000afe86c660a568a750c603f72dba13b32abb1f31125188"),
        0,
    )
}

// bitcoin mainnet
pub fn mainnet() -> BitcoinGenesisInfo {
    (
        (
            BlockHeader {
                version: 536870912,
                previous_header_hash: h256_from_rev_str(
                    "0000000000000000000a4adf6c5192128535d4dcb56cfb5753755f8d392b26bf",
                ),
                merkle_root_hash: h256_from_rev_str(
                    "1d21e60acb0b12e5cfd3f775edb647f982a2d666f9886b2f61ea5e72577b0f5e",
                ),
                time: 1558168296,
                bits: Compact::new(388627269),
                nonce: 1439505020,
            },
            576576,
        ),
        h256_from_rev_str("0000000000000000001721f58deb88b0710295a02551f0dde1e2e231a15f1882"),
        0,
    )
}
