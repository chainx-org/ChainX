// Copyright 2018-2019 Chainpool.

#![cfg(test)]

mod header;
mod lockup;
mod mock;
mod opreturn;
mod trustee;

use self::mock::*;
use super::*;

use hex_literal::hex;

use runtime_io::with_externalities;
use substrate_primitives::crypto::UncheckedInto;
use support::StorageValue;
use support::{assert_err, assert_ok};

use btc_crypto::dhash160;
use btc_keys::DisplayLayout;
use btc_primitives::{h256_from_rev_str, Compact};
use btc_script::{Builder, Opcode, Script};

fn reverse_h256(mut hash: btc_primitives::H256) -> btc_primitives::H256 {
    let bytes = hash.as_bytes_mut();
    bytes.reverse();
    btc_primitives::H256::from_slice(bytes)
}

fn current_time() -> u64 {
    use std::time;
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("now always later than unix epoch; qed")
        .as_secs()
}

#[test]
pub fn test_address() {
    XBridgeOfBTC::verify_btc_address(&b"mqVznxoxdeSNYgDCg6ZVE5pc6476BY6zHK".to_vec()).unwrap();
}

#[test]
fn test_accountid() {
    let script = Script::from(
        "5HnDcuKFCvsR42s8Tz2j2zLHLZAaiHG4VNyJDa7iLRunRuhM@33"
            .as_bytes()
            .to_vec(),
    );
    let s = script.to_bytes();
    let mut iter = s.as_slice().split(|x| *x == '@' as u8);
    let mut v = Vec::new();
    while let Some(d) = iter.next() {
        v.push(d);
    }
    assert_eq!(v.len(), 2);
    let mut slice: Vec<u8> = b58::from(v[0]).unwrap();
    let account_id: H256 = Decode::decode(&mut slice[1..33].to_vec().as_slice()).unwrap();
    let bytes = hex!("fcd66b3b5a737f8284fef82d377d9c2391628bbe11ec63eb372b032ce2618725");
    assert_eq!(account_id, H256::from_slice(&bytes));
}

//#[test]
//fn test_sign_withdraw() {
//    with_externalities(&mut new_test_ext(), || {
//        let _tx1 = hex::decode("01000000019d15247f7f75ffd6e9377ea928f476bcaf9ab542563429b97ee2ef89f2c9d4a101000000b5004830450221008c9147795b2ddf923d5dad3c9fcfde6394aa2629b9a10ca8f93a5c6d4293a7490220687aeb3318b35450fda4d45cc54177f3d6f898d15ea1f8705a77c7116cb44fe8014c695221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d2102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae4053aeffffffff01e8030000000000001976a914023dbd259dd15fc43da1a758ea7b2bfaec97893488ac00000000").unwrap();
//        let _tx = hex::decode("01000000019d15247f7f75ffd6e9377ea928f476bcaf9ab542563429b97ee2ef89f2c9d4a101000000fdfd00004830450221008c9147795b2ddf923d5dad3c9fcfde6394aa2629b9a10ca8f93a5c6d4293a7490220687aeb3318b35450fda4d45cc54177f3d6f898d15ea1f8705a77c7116cb44fe80147304402204b999fbf18b944a3f6446ca56d094d70699a1e44c8636b06fc2267434e9200ae022073327aca6cdad35075c9c8bb2759a24753906ef030ccb513d8a515648ab46d0e014c695221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d2102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae4053aeffffffff01e8030000000000001976a914023dbd259dd15fc43da1a758ea7b2bfaec97893488ac00000000").unwrap();
//        let _redeem_script: Script = Script::from("532103f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba210306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40210227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c354ae");
//        //        handle_condidate::<Test>(tx).unwrap();
//    })
//}
