// Copyright 2018-2020 Chainpool.

use super::*;
use hex::FromHex;

#[derive(Debug, Deserialize)]
pub struct RecordOfSDOT {
    tx_hash: String,
    block_number: u64,
    unix_timestamp: u64,
    date_time: String,
    from: String,
    to: String,
    quantity: f64,
}

#[allow(clippy::type_complexity)]
pub fn load_genesis() -> Result<Vec<([u8; 20], u64)>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_reader(&include_bytes!("res/dot_tx.csv")[..]);
    let mut res = Vec::with_capacity(3052);
    for result in reader.deserialize() {
        let record: RecordOfSDOT = result?;
        let sdot_addr = <[u8; 20] as FromHex>::from_hex(&record.to[2..])?;
        res.push((sdot_addr, (record.quantity * 1000.0).round() as u64));
    }
    Ok(res)
}

pub fn create_asset() -> Asset {
    Asset::new(
        b"SDOT".to_vec(), // token
        b"Shadow DOT".to_vec(),
        Chain::Ethereum,
        3, //  precision
        b"ChainX's Shadow Polkadot from Ethereum".to_vec(),
    )
    .unwrap()
}

#[test]
fn test_quantity_sum() {
    let res = load_genesis().unwrap();
    let sum: u64 = res.iter().map(|(_, quantity)| *quantity).sum();
    assert_eq!(sum, 4999466375u64);
}
