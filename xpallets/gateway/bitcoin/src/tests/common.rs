// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub use codec::{Decode, Encode};
#[cfg(test)]
pub use frame_support::{assert_noop, assert_ok};
pub use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub use light_bitcoin::{
    primitives::H256,
    script::{Builder, Opcode, Script},
    serialization,
};

#[cfg(test)]
pub use super::mock::*;
pub use crate::*;

pub fn reverse_h256(mut hash: H256) -> H256 {
    let bytes = hash.as_bytes_mut();
    bytes.reverse();
    H256::from_slice(bytes)
}

#[cfg(test)]
pub fn as_h256(s: &str) -> H256 {
    h256_conv_endian_from_str(s)
}

#[cfg(test)]
pub fn generate_blocks() -> BTreeMap<u32, BtcHeader> {
    let bytes = include_bytes!("./res/headers-576576-578692.json");
    let headers: Vec<(u32, String)> = serde_json::from_slice(&bytes[..]).expect("should not fail");
    headers
        .into_iter()
        .map(|(height, h)| {
            let hex = hex::decode(h).expect("should be valid hex");
            let header =
                serialization::deserialize(Reader::new(&hex)).expect("should be valid header");
            (height, header)
        })
        .collect()
}
#[cfg(feature = "runtime-benchmarks")]
pub fn generate_blocks_from_raw() -> BTreeMap<u32, BtcHeader> {
    let bytes = include_bytes!("./res/headers-576576-578692.raw");
    Decode::decode(&mut &bytes[..]).expect("must decode success")
}

#[cfg(test)]
const PUBKEYS: [([u8; 33], [u8; 33]); 3] = [
    (
        [
            2, 223, 146, 232, 140, 67, 128, 119, 140, 156, 72, 38, 132, 96, 161, 36, 168, 244, 231,
            218, 136, 63, 128, 71, 125, 234, 166, 68, 206, 212, 134, 239, 198,
        ],
        [
            3, 134, 181, 143, 81, 218, 155, 55, 229, 156, 64, 38, 33, 83, 23, 59, 219, 89, 215,
            228, 228, 91, 115, 153, 75, 153, 238, 196, 217, 100, 238, 126, 136,
        ],
    ),
    (
        [
            2, 68, 216, 30, 254, 180, 23, 27, 26, 138, 67, 59, 135, 221, 32, 33, 23, 249, 78, 68,
            201, 9, 196, 158, 66, 231, 123, 105, 181, 166, 206, 125, 13,
        ],
        [
            2, 228, 99, 30, 70, 37, 85, 113, 18, 45, 110, 17, 205, 167, 93, 93, 96, 29, 94, 178,
            88, 94, 101, 228, 232, 127, 233, 246, 140, 120, 56, 162, 120,
        ],
    ),
    (
        [
            3, 163, 99, 57, 244, 19, 218, 134, 157, 241, 43, 26, 176, 222, 249, 23, 73, 65, 58, 13,
            238, 135, 240, 191, 168, 91, 167, 25, 110, 108, 218, 209, 2,
        ],
        [
            2, 99, 212, 108, 118, 13, 62, 4, 136, 61, 75, 67, 60, 156, 226, 188, 50, 19, 10, 205,
            159, 170, 208, 25, 42, 43, 55, 93, 187, 169, 248, 101, 195,
        ],
    ),
];

pub fn accounts<T: Trait>() -> [T::AccountId; 3] {
    // sr25519 generate pubkey
    let alice = [
        212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88,
        133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
    ];

    let bob = [
        142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161, 126, 37, 252, 82, 135, 97, 54, 147,
        201, 18, 144, 156, 178, 38, 170, 71, 148, 242, 106, 72,
    ];

    let charlie = [
        144, 181, 171, 32, 92, 105, 116, 201, 234, 132, 27, 230, 136, 134, 70, 51, 220, 156, 168,
        163, 87, 132, 62, 234, 207, 35, 20, 100, 153, 101, 254, 34,
    ];
    let alice: AccountId32 = alice.into();
    let bob: AccountId32 = bob.into();
    let charlie: AccountId32 = charlie.into();

    let a = alice.encode();
    let alice = T::AccountId::decode(&mut &a[..]).unwrap();
    let b = bob.encode();
    let bob = T::AccountId::decode(&mut &b[..]).unwrap();
    let c = charlie.encode();
    let charlie = T::AccountId::decode(&mut &c[..]).unwrap();
    [alice, bob, charlie]
}

#[cfg(test)]
pub fn trustees<T: Trait>() -> Vec<(T::AccountId, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let accounts = accounts::<T>();
    let btc_trustees = vec![
        (
            accounts[0].clone(),
            b"".to_vec(),
            PUBKEYS[0].0.to_vec(),
            PUBKEYS[0].1.to_vec(),
        ),
        (
            accounts[1].clone(),
            b"".to_vec(),
            PUBKEYS[1].0.to_vec(),
            PUBKEYS[1].1.to_vec(),
        ),
        (
            accounts[2].clone(),
            b"".to_vec(),
            PUBKEYS[2].0.to_vec(),
            PUBKEYS[2].1.to_vec(),
        ),
    ];
    btc_trustees
}

// #[test]
// #[ignore]
// fn tmp_generate_raw_headers_file() {
//     use codec::Encode;
//     let raw_headers = generate_blocks();
//     let bytes = raw_headers.encode();
//     // rep
//     std::fs::write("/home/king/workspace/chainx-org/ChainX/xpallets/gateway/bitcoin/src/tests/res/headers-576576-578692.raw", bytes).unwrap();
// }

#[cfg(test)]
pub mod for_tests {
    use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};
    use std::sync::Mutex;

    pub struct Guard<'a>((std::sync::MutexGuard<'a, ()>, Ss58AddressFormat));

    impl<'a> Drop for Guard<'a> {
        fn drop(&mut self) {
            set_default_ss58_version((self.0).1)
        }
    }
    lazy_static::lazy_static!(
        static ref LOCK: Mutex<()> = Mutex::new(());
    );
    pub fn force_ss58_version() -> Guard<'static> {
        let c = LOCK.lock().unwrap();
        let default = Ss58AddressFormat::default();
        set_default_ss58_version(Ss58AddressFormat::ChainXAccount);
        Guard((c, default))
    }
}

#[cfg(test)]
pub use for_tests::force_ss58_version;
use sp_runtime::AccountId32;
