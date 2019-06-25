// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
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
fn test() {
    with_externalities(&mut new_test_ext(), || {
        use substrate_primitives::hexdisplay::HexDisplay;
        let r = <BlockHeaderFor<Test>>::key_for(&h256_from_rev_str(
            "00000000000025c23a19cc91ad8d3e33c2630ce1df594e1ae0bf0eabe30a9176",
        ));
        let a = substrate_primitives::twox_128(&r);
        println!("0x{:}", HexDisplay::from(&a));
    })
}

#[test]
fn test_init_blocks() {
    let (c1, _) = generate_blocks();

    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(0).unwrap().hash())),
        "0x2c22ca732c7b99c43057df342f903ffc8a7e132e09563edb122b1f573458ac5b"
    );
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(1).unwrap().hash())),
        "0x0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd"
    );
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(2).unwrap().hash())),
        "0x00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe"
    );
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(3).unwrap().hash())),
        "0x000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8"
    );
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(4).unwrap().hash())),
        "0x00000000000000e83086b78ebc3da4af6d892963fa3fd5e1648c693de623d1b7"
    );
}

#[test]
fn test_init_mock_blocks() {
    let (c1, _) = generate_mock_blocks();
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(0).unwrap().hash())),
        "0x2c22ca732c7b99c43057df342f903ffc8a7e132e09563edb122b1f573458ac5b"
    );
    println!("{:?}", btc_ser::serialize(c1.get(1).unwrap()));
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(1).unwrap().hash())),
        "0x0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd"
    );
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(2).unwrap().hash())),
        "0x00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe"
    );
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(3).unwrap().hash())),
        "0x000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8"
    );
    assert_eq!(
        format!("{:?}", reverse_h256(c1.get(4).unwrap().hash())),
        "0x00000000000000e83086b78ebc3da4af6d892963fa3fd5e1648c693de623d1b7"
    );
}

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        let (header, num) = XBridgeOfBTC::genesis_info();
        let _r = <GenesisInfo<Test>>::get();
        assert_eq!(
            format!("{:?}", reverse_h256(header.hash())),
            "0x00000000000000fd9cea8b846895f507c63b005d20ac56e87d1cdf80effd5c0a"
        );
        assert_eq!(num, 1451572);

        let best_hash = XBridgeOfBTC::best_index();
        assert_eq!(best_hash, header.hash());
    })
}

#[test]
fn test_err_genesis_startnumber() {
    with_externalities(&mut new_test_ext_err_genesisblock(), || {})
}

#[test]
fn test_normal() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        assert_err!(
            XBridgeOfBTC::apply_push_header(c1.get(0).unwrap().clone()),
            "Block parent is unknown"
        );
        assert_ok!(XBridgeOfBTC::apply_push_header(c1.get(1).unwrap().clone()));
        assert_ok!(XBridgeOfBTC::apply_push_header(c1.get(2).unwrap().clone()));

        let best_hash = XBridgeOfBTC::best_index();
        assert_eq!(best_hash, c1.get(2).unwrap().hash());
    })
}

#[test]
fn test_call() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        let origin = system::RawOrigin::Signed(99).into();
        let v = btc_ser::serialize(c1.get(1).unwrap());
        let v = v.take();
        assert_ok!(XBridgeOfBTC::push_header(origin, v));
    })
}

#[test]
fn test_genesis2() {
    with_externalities(&mut new_test_ext2(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        assert_err!(
            XBridgeOfBTC::apply_push_header(c1.get(0).unwrap().clone()),
            "Block parent is unknown"
        );
        assert_ok!(XBridgeOfBTC::apply_push_header(c1.get(1).unwrap().clone()));
        assert_ok!(XBridgeOfBTC::apply_push_header(c1.get(2).unwrap().clone()));
        assert_ok!(XBridgeOfBTC::apply_push_header(c1.get(3).unwrap().clone()));
    })
}

#[test]
fn test_changebit() {
    with_externalities(&mut new_test_ext2(), || {
        let b1 = BlockHeader {
            version: 1,
            previous_header_hash: h256_from_rev_str(
                "00000000864b744c5025331036aa4a16e9ed1cbb362908c625272150fa059b29",
            ),
            merkle_root_hash: h256_from_rev_str(
                "70d6379650ac87eaa4ac1de27c21217b81a034a53abf156c422a538150bd80f4",
            ),
            time: 1337966314,
            bits: Compact::new(486604799),
            nonce: 2391008772,
        };
        // 2016
        assert_eq!(
            format!("{:?}", reverse_h256(b1.hash())),
            "0x0000000089d757fd95d79f7fcc2bc25ca7fc16492dca9aa610730ea05d9d3de9"
        );

        let _b2 = BlockHeader {
            version: 1,
            previous_header_hash: h256_from_rev_str(
                "00000000864b744c5025331036aa4a16e9ed1cbb362908c625272150fa059b29",
            ),
            merkle_root_hash: h256_from_rev_str(
                "70d6379650ac87eaa4ac1de27c21217b81a034a53abf156c422a538150bd80f4",
            ),
            time: 1337966314,
            bits: Compact::new(486604799),
            nonce: 2391008772,
        };
        // 2017
        assert_eq!(
            format!("{:?}", reverse_h256(b1.hash())),
            "0x0000000089d757fd95d79f7fcc2bc25ca7fc16492dca9aa610730ea05d9d3de9"
        );
    })
}

#[test]
pub fn test_address() {
    XBridgeOfBTC::verify_btc_address(&b"mqVznxoxdeSNYgDCg6ZVE5pc6476BY6zHK".to_vec()).unwrap();
}

#[test]
pub fn test_multi_address() {
    let pub1 = String::from("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let pub2 = String::from("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2");
    let pub3 = String::from("023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d");

    let pubkey1_bytes = hex::decode(pub1).unwrap();
    let pubkey2_bytes = hex::decode(pub2).unwrap();
    let pubkey3_bytes = hex::decode(pub3).unwrap();

    let script = Builder::default()
        .push_opcode(Opcode::OP_2)
        .push_bytes(&pubkey1_bytes)
        .push_bytes(&pubkey2_bytes)
        .push_bytes(&pubkey3_bytes)
        .push_opcode(Opcode::OP_3)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();
    //let test = hex_script!("52210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d53ae");
    let multisig_address = btc_keys::Address {
        kind: btc_keys::Type::P2SH,
        network: btc_keys::Network::Testnet,
        hash: dhash160(&script),
    };
    assert_eq!(
        "2MtAUgQmdobnz2mu8zRXGSTwUv9csWcNwLU",
        multisig_address.to_string()
    );
}

fn create_multi_address(pubkeys: Vec<Vec<u8>>) -> btc_keys::Address {
    let mut build = Builder::default().push_opcode(Opcode::OP_3);
    for (_, pubkey) in pubkeys.iter().enumerate() {
        build = build.push_bytes(pubkey);
    }
    let script = build
        .push_opcode(Opcode::OP_4)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();
    let multisig_address = btc_keys::Address {
        kind: btc_keys::Type::P2SH,
        network: btc_keys::Network::Testnet,
        hash: dhash160(&script),
    };
    multisig_address
}

#[test]
fn test_create_multi_address() {
    //hot
    let pubkey1_bytes =
        hex::decode("03f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba").unwrap();
    let pubkey2_bytes =
        hex::decode("0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd").unwrap();
    let pubkey3_bytes =
        hex::decode("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40").unwrap();
    let pubkey4_bytes =
        hex::decode("0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3").unwrap();

    //cold
    let pubkey5_bytes =
        hex::decode("02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee").unwrap();
    let pubkey6_bytes =
        hex::decode("03ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d70780").unwrap();
    let pubkey7_bytes =
        hex::decode("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2").unwrap();
    let pubkey8_bytes =
        hex::decode("020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f").unwrap();
    if pubkey4_bytes.len() != 33 && pubkey4_bytes.len() != 65 {
        panic!("pubkey_bytes error 2")
    }
    let mut hot_keys = Vec::new();
    let mut cold_keys = Vec::new();
    hot_keys.push(pubkey1_bytes);
    hot_keys.push(pubkey2_bytes);
    hot_keys.push(pubkey3_bytes);
    hot_keys.push(pubkey4_bytes);

    cold_keys.push(pubkey5_bytes);
    cold_keys.push(pubkey6_bytes);
    cold_keys.push(pubkey7_bytes);
    cold_keys.push(pubkey8_bytes);
    //hot_keys.sort();

    let _hot_addr = create_multi_address(hot_keys);
    let cold_addr = create_multi_address(cold_keys);

    let cold_layout_addr = cold_addr.layout().to_vec();
    let layout = [
        196, 96, 201, 52, 180, 27, 175, 109, 29, 168, 76, 211, 20, 252, 208, 243, 210, 16, 105, 83,
        0, 42, 109, 109, 135,
    ];

    assert_eq!(cold_layout_addr, layout);

    let addr = btc_keys::Address::from_layout(&mut cold_layout_addr.as_slice()).unwrap();

    assert_eq!(cold_addr, addr);

    let pks = [
        169, 20, 96, 201, 52, 180, 27, 175, 109, 29, 168, 76, 211, 20, 252, 208, 243, 210, 16, 105,
        83, 0, 135,
    ];
    let pk = _hot_addr.hash.clone().as_bytes().to_vec();
    let mut pubkeys = Vec::new();
    pubkeys.push(Opcode::OP_HASH160 as u8);
    pubkeys.push(Opcode::OP_PUSHBYTES_20 as u8);
    for p in pk {
        pubkeys.push(p)
    }
    pubkeys.push(Opcode::OP_EQUAL as u8);
    assert_eq!(pubkeys, pks);
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
    let bytes =
        hex::decode("fcd66b3b5a737f8284fef82d377d9c2391628bbe11ec63eb372b032ce2618725").unwrap();
    assert_eq!(account_id, H256::from_slice(&bytes));
}

#[test]
fn test_sign_withdraw() {
    with_externalities(&mut new_test_ext3(), || {
        let _tx1 = hex::decode("01000000019d15247f7f75ffd6e9377ea928f476bcaf9ab542563429b97ee2ef89f2c9d4a101000000b5004830450221008c9147795b2ddf923d5dad3c9fcfde6394aa2629b9a10ca8f93a5c6d4293a7490220687aeb3318b35450fda4d45cc54177f3d6f898d15ea1f8705a77c7116cb44fe8014c695221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d2102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae4053aeffffffff01e8030000000000001976a914023dbd259dd15fc43da1a758ea7b2bfaec97893488ac00000000").unwrap();
        let _tx = hex::decode("01000000019d15247f7f75ffd6e9377ea928f476bcaf9ab542563429b97ee2ef89f2c9d4a101000000fdfd00004830450221008c9147795b2ddf923d5dad3c9fcfde6394aa2629b9a10ca8f93a5c6d4293a7490220687aeb3318b35450fda4d45cc54177f3d6f898d15ea1f8705a77c7116cb44fe80147304402204b999fbf18b944a3f6446ca56d094d70699a1e44c8636b06fc2267434e9200ae022073327aca6cdad35075c9c8bb2759a24753906ef030ccb513d8a515648ab46d0e014c695221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d2102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae4053aeffffffff01e8030000000000001976a914023dbd259dd15fc43da1a758ea7b2bfaec97893488ac00000000").unwrap();
        let _redeem_script: Script = Script::from("532103f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba210306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40210227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c354ae");
        //        handle_condidate::<Test>(tx).unwrap();
    })
}

#[test]
fn test_sign_state() {
    let mut data = Vec::new();
    data.push((1, true));
    data.push((2, true));
    data.push((3, true));
    data.push((4, true));
    let vote_state = false;
    insert_trustee_vote_state::<Test>(vote_state, &1, &mut data);
    insert_trustee_vote_state::<Test>(vote_state, &3, &mut data);
    insert_trustee_vote_state::<Test>(vote_state, &2, &mut data);
    insert_trustee_vote_state::<Test>(vote_state, &4, &mut data);
    let d = vec![(1, false), (3, false), (2, false), (4, false)];
    assert_eq!(data, d);
}
