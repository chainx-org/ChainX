// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

// mod header;
// mod opreturn;
// mod others;
mod trustee;
// mod tx;

use std::collections::BTreeMap;

use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    serialization::{deserialize, Reader},
};

pub fn generate_blocks() -> BTreeMap<u32, BtcHeader> {
    let headers = include_str!("../res/headers-576576-578692.json");
    let headers: Vec<(u32, String)> = serde_json::from_str(headers).unwrap();
    headers
        .into_iter()
        .map(|(height, header_hex)| {
            let data = hex::decode(header_hex).unwrap();
            let header = deserialize(Reader::new(&data)).unwrap();
            (height, header)
        })
        .collect()
}
