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

pub fn load_mainnet_btc_genesis_header_info(
) -> ((BtcHeader, u32), xpallet_gateway_bitcoin::H256, BtcNetwork) {
    let raw: BitcoinGenesisHeader =
        serde_json::from_str(include_str!("./res/btc_genesis_header_mainnet.json"))
            .expect("JSON was not well-formatted");
    let as_h256 = |s: &str| h256_conv_endian_from_str(s);
    (
        (
            BtcHeader {
                version: raw.version,
                previous_header_hash: as_h256(&raw.previous_header_hash),
                merkle_root_hash: as_h256(&raw.previous_header_hash),
                time: raw.time,
                bits: BtcCompact::new(raw.bits),
                nonce: raw.nonce,
            },
            raw.height,
        ),
        as_h256(&raw.hash),
        as_btc_network(&raw.network_id),
    )
}
