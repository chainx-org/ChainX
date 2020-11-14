// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

// mod header;
// pub mod mock;
// mod opreturn;
// mod others;
// mod trustee;
// mod tx;

use codec::{Decode, Encode};

use sp_runtime::AccountId32;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    serialization::{deserialize, Reader},
};

#[cfg(test)]
pub fn generate_blocks() -> BTreeMap<u32, BtcHeader> {
    let headers = include_str!("./res/headers-576576-578692.json");
    let headers: Vec<(u32, String)> = serde_json::from_str(headers).unwrap();
    headers
        .into_iter()
        .map(|(height, header_hex)| {
            let bytes = hex::decode(header_hex).unwrap();
            let header = deserialize(Reader::new(&bytes)).unwrap();
            (height, header)
        })
        .collect()
}

#[cfg(test)]
pub fn accounts() -> [AccountId32; 3] {
    [
        sp_keyring::sr25519::Keyring::Alice.to_account_id(),
        sp_keyring::sr25519::Keyring::Bob.to_account_id(),
        sp_keyring::sr25519::Keyring::Charlie.to_account_id(),
    ]
}

#[cfg(test)]
pub fn trustees() -> Vec<(AccountId32, Vec<u8>, Vec<u8>, Vec<u8>)> {
    use hex_literal::hex;
    vec![
        (
            sp_keyring::sr25519::Keyring::Alice.to_account_id(),
            b"Alice".to_vec(),
            hex!("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6").to_vec(),
            hex!("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88").to_vec(),
        ),
        (
            sp_keyring::sr25519::Keyring::Bob.to_account_id(),
            b"Bob".to_vec(),
            hex!("0244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d").to_vec(),
            hex!("02e4631e46255571122d6e11cda75d5d601d5eb2585e65e4e87fe9f68c7838a278").to_vec(),
        ),
        (
            sp_keyring::sr25519::Keyring::Charlie.to_account_id(),
            b"Charlie".to_vec(),
            hex!("03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102").to_vec(),
            hex!("0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3").to_vec(),
        ),
    ]
}
