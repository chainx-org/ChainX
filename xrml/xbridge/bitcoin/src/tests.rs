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
pub fn test_check_trustee_entity() {
    let addr_ok_3 =
        hex::decode("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40").unwrap();
    let public3 = Public::from_slice(&addr_ok_3).map_err(|_| "Invalid Public");
    assert_eq!(XBridgeOfBTC::check_trustee_entity(&addr_ok_3), public3);

    let addr_ok_2 =
        hex::decode("0211252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40").unwrap();
    let public2 = Public::from_slice(&addr_ok_2).map_err(|_| "Invalid Public");
    assert_eq!(XBridgeOfBTC::check_trustee_entity(&addr_ok_2), public2);

    let addr_too_long =
        hex::decode("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40cc")
            .unwrap();
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_too_long),
        Err("Invalid Public")
    );

    let addr_normal=hex::decode("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae4011252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40").unwrap();
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_normal),
        Err("not allow Normal Public for bitcoin now")
    );

    let addr_err_type =
        hex::decode("0411252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40").unwrap();
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_err_type),
        Err("not Compressed Public(2|3)")
    );

    let addr_zero =
        hex::decode("020000000000000000000000000000000000000000000000000000000000000000").unwrap();
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_zero),
        Err("not Compressed Public(Zero32)")
    );

    let addr_EC_P =
        hex::decode("02fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f").unwrap();
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_EC_P),
        Err("not Compressed Public(EC_P)")
    );

    let addr_EC_P_2 =
        hex::decode("02fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc3f").unwrap();
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_EC_P_2),
        Err("not Compressed Public(EC_P)")
    );
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

//
#[test]
fn test_opreturn() {
    // error tx from mathwallet test
    // pubkey just have opreturn
    // txid e41061d3ad1d6a46c69be30475e23446cccf1a05e4dc9eaf6bc33443e51b0f2f (witness)
    let t1: Transaction = "020000000001011529f2fbaca4cc374e12409cc3db0a8fe2509894f8b79f1f67d648f488d7a1f50100000017160014b1ef3d9fd4a68b53e75c56845076bfb4b4ae3974ffffffff03307500000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587bfe400000000000017a9141df425d522de50d46c32f979d73b823887446fd0870000000000000000016a02483045022100d591090fd8f0d62145d967fad754533fcdb5e7180c8644d16d071c3c5dfcb3a802200ee6cea9eb146d7e24b4142c36baa19e9c4c70095ef9b3ccc736247ecf0b8ed3012102632394028f212c1bc88f01dd14b4f8bc81c16ef464c830021030062a8f7788ae00000000".into();
    println!("{:?}", t1);
    // txid f5a1d788f448d6671f9fb7f8949850e28f0adbc39c40124e37cca4acfbf22915 (witness)
    let t2: Transaction = "02000000000101681bd0b1158c7dc4ade8818c20820bedb906773a48c614e6ddc44cfd3c37408f010000001716001485863aa315bc11a844bc1eee01547be6a302a7caffffffff03204e00000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458717a501000000000017a914d5ea60928669d832351b023bcfb3e85c530817d9870000000000000000016a02483045022100be53337e0c816e4f4d61b8b535431199105f04a1c043bd1d0f0362a525d7678502204ec154badbc84435d0c059b742dfddccca6338042fbf7e77bbfdbbfba183e1a10121025eb9e1c63f28cccc67739ee940256fc26259e06167a0e9c411023bb1377ab1a000000000".into();
    println!("{:?}", t2);

    // opreturn with 80 bytes
    let t3: Transaction = "0200000001776ae4d3fbebbd8568c610b265f54a1a8e1f03f2a16cac99ca9490e32583313b000000006a473044022074edd3b4f333ba3b0edb685922420bf904d417cd24584dbe76ad2e9b9c54e37602202a4027f77b7a4f6aaa7a8e7423e0b4740531e7a97527d51f341f75a950480b7f012102ebaf854b6220e3d44a32373aabbe1b6e4c3f824a7855aeac65b6854cd84d6f87ffffffff02a0bb0d00000000001976a9146ffd34b262b5099b80f8e84fe7e5dccaa79e2e7a88ac0000000000000000536a4c50999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999900000000".into();
    println!("{:?}", t3);

    // opreturn normal
    // opreturn normal with addr (5Uj3ehamDZWPfgA8iAZenhcAmPDakjf4aMbkBB4dXVvjoW6x) (witness)
    // txid: b368d3b822ec6656af441ccfa0ea2c846ec445286fd264e94a9a6edf0d7a1108
    let t4: Transaction = "020000000001012f0f1be54334c36baf9edce4051acfcc4634e27504e39bc6466a1dadd36110e40100000017160014cd286c8c974540b1019e351c33551dc152e7447bffffffff03307500000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587672400000000000017a9149b995c9fddc8e5086626f7123631891a209d83a4870000000000000000326a3035556a336568616d445a57506667413869415a656e6863416d5044616b6a6634614d626b424234645856766a6f57367802483045022100f27347145406cc9706cd4d83018b07303c30b8d43f935019bf1d3accb38696f70220546db7a30dc8f0c4f02e17460573d009d26d85bd98a32642e88c6f74e76ac7140121037788522b753d5517cd9191c96f741a0d2b479369697d41567b4b418c7979d77300000000".into();
    println!("{:?}", t4);

    // opreturn normal with addr and channel (5QWKZY4QAt4NC8s5qcJVJnSbLSJ1W9iv5S4iJJPUr3Pdkdnj@Axonomy)
    // txid: a7c91cb83ec0c0182704cafc447a2eb075c29d7d809b4898cd4aa37324f2b770
    let t5: Transaction = "020000000386389a63d8e858e06236d2b8de206763f2bd858adcbc8deb03bdb1f673b0d19c040000006b483045022100a4f40ddc02bb0326f476e664ac08015e4fd157c545dc2d03933e037b0b380f0e0220653f2fc0c229d3ce73f0829b53007700d6c517d27bcfdd1ad6ebdfce4fcbf1c20121024bfe28c0f47d7913d3fbd4555a63d448529924332d76c3b66251c9cd4ffa8340000000004e82355663aae88d258871ceff235a9c743291e3b1e1f4c2db6dd0774fe8ec8d010000006a473044022030013c331cbaa3a34a827d3c6a02e9dc93a88ef8ecb63a3d33b5c3087bcb8c7702205808f28435a7f22d30bb9540bafc58f2f0a4e2c3e0e5cc6ab59a2c7fbdfd9a610121024bfe28c0f47d7913d3fbd4555a63d448529924332d76c3b66251c9cd4ffa834000000000bd9bb637bc1e3bfa6209abeb59bdfd24aa1e80d911a00762a467a2488b4ba7fd000000006b483045022100bccff95c3298dd74027e5aa65da216384754136dee8b578cd6e70c7c3d19964d022078d71696e92a41d7d228b94020035b102cc3d4958dee2357c7aeeb509561678d0121024bfe28c0f47d7913d3fbd4555a63d448529924332d76c3b66251c9cd4ffa8340000000000380d99f380000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587c0d40100000000001976a9146e9557e4fce7b1bb47056e357811c51b165ff8f488ac00000000000000003a6a383551574b5a5934514174344e4338733571634a564a6e53624c534a3157396976355334694a4a5055723350646b646e6a4041786f6e6f6d7900000000".into();
    println!("{:?}", t5);

    // opreturn normal with addr and channel (5TtJf6MVyCcmS4SGh35SLzbhA76U5rNdURqZuVhjetsEKRND@MathWallet) (witness)
    // txid: 41a5dedd90caa452fda70d50adfe9ce69c6ca75e05bfb8c5a4b426fda29436ad
    let t6: Transaction = "01000000000101b3dce032c6e5f6dd88f39f4197d76cf0b66b7592fdda7ba3e02bcebff9df7a7e010000001716001485863aa315bc11a844bc1eee01547be6a302a7caffffffff0300000000000000003d6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c6574f82a00000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587788f03000000000017a914d5ea60928669d832351b023bcfb3e85c530817d98702483045022100a16ac5ceb9ed9bb4aa8099fa5c8e8758e6ade55d2347c1d81c98550156900cb8022030e2b3c3e061ae353770b351c976ec9712a29608cf982d3a42daa2fa5329e6ea0121025eb9e1c63f28cccc67739ee940256fc26259e06167a0e9c411023bb1377ab1a000000000".into();
    println!("{:?}", t6);

    // opreturn normal with addr and channel (5QSHP7aZaW35N88qf7JHJAYZQBkxpMfRpeSBpaj3NT1HMDtn)
    // txid: 9dee96445c3c7e9f2f215e009a3fada6118b5d8d0f5824431fd90bdde3ee72bb
    let t7: Transaction = "010000000199ada0c9b227557545aee0a5c948db96b8f009c8e57ba113af5d811fb51306fd000000006a473044022001eb5c5eb0852063e9cbea6d2d92b76b14998bef21af2231280b10a7df0abce80220497d3f8ba4e2c10b23dcff61b6d6c0e8179da0de9a675f81fc3685b5330ff158012103cf3e8985580fb495bddbb3baae07c35f2237da7e3d1a8e853cb2080ba6fa6ca4ffffffff03102700000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587710c0000000000001976a9140c456455ffdb307bd046ac4def9ee6522c54e24888ac0000000000000000326a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e00000000".into();
    println!("{:?}", t7);

    // error tx
    let t8: Transaction = "0200000001776ae4d3fbebbd8568c610b265f54a1a8e1f03f2a16cac99ca9490e32583313b000000006a47304402201871b85a7f608a24bcb95d3c8beeddef2d33377a6956d75d534faf3bca4d4fc102200ad4683ccad758f1f9de1e9d5a6af6d521010778bab4ded856eb4689355f670b012102ebaf854b6220e3d44a32373aabbe1b6e4c3f824a7855aeac65b6854cd84d6f87ffffffff030000000000000000536a4c509999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999a0bb0d000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000000000003d6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c657400000000".into();
    println!("{:?}", t8);

    let t9: Transaction = "0200000001776ae4d3fbebbd8568c610b265f54a1a8e1f03f2a16cac99ca9490e32583313b000000006b483045022100e7526da20fda326cce8181516906fc287c49c6f420843f2ecdb0ee4d72e6f899022053259e1e4e6fea0be0277ec1f5c21822c678ac8999887369c4b05c0f897eae81012102ebaf854b6220e3d44a32373aabbe1b6e4c3f824a7855aeac65b6854cd84d6f87ffffffff03a0bb0d000000000017a914cb94110435d0635223eebe25ed2aaabc03781c45870000000000000000326a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e00000000000000003d6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c657400000000".into();
    println!("{:?}", t9);

    with_externalities(&mut new_test_mainnet(), || {
        use hex::FromHex;
        use tx::handler::parse_deposit_outputs_impl;

        let hot_addr =
            XBridgeOfBTC::verify_btc_address(b"3LFSUKkP26hun42J1Dy6RATsbgmBJb27NF").unwrap();
        println!("{:?}", hot_addr);

        let r = parse_deposit_outputs_impl::<Test>(&t1, &hot_addr).unwrap();
        assert_eq!(r, (None, 30000, None));

        let r = parse_deposit_outputs_impl::<Test>(&t2, &hot_addr).unwrap();
        assert_eq!(r, (None, 20000, None));

        let r = parse_deposit_outputs_impl::<Test>(&t3, &hot_addr).unwrap();
        //        assert_eq!(r, (Some((999, None)), 0, Some(Vec::from_hex("6a4c509999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999").unwrap())));
        assert_eq!(r, (None, 0, None));

        let r = parse_deposit_outputs_impl::<Test>(&t4, &hot_addr).unwrap();
        assert_eq!(r, (Some((999, None)), 30000, Some(Vec::from_hex("6a3035556a336568616d445a57506667413869415a656e6863416d5044616b6a6634614d626b424234645856766a6f573678").unwrap())));

        let r = parse_deposit_outputs_impl::<Test>(&t5, &hot_addr).unwrap();
        assert_eq!(r, (Some((999, Some("Axonomy".as_bytes().to_vec()))), 950000000, Some(Vec::from_hex("6a383551574b5a5934514174344e4338733571634a564a6e53624c534a3157396976355334694a4a5055723350646b646e6a4041786f6e6f6d79").unwrap())));

        let r = parse_deposit_outputs_impl::<Test>(&t6, &hot_addr).unwrap();
        assert_eq!(r, (Some((999, Some("MathWallet".as_bytes().to_vec()))), 11000, Some(Vec::from_hex("6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c6574").unwrap())));

        let r = parse_deposit_outputs_impl::<Test>(&t7, &hot_addr).unwrap();
        assert_eq!(r, (Some((999, None)), 10000, Some(Vec::from_hex("6a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e").unwrap())));

        let r = parse_deposit_outputs_impl::<Test>(&t8, &hot_addr).unwrap();
        assert_eq!(r, (Some((999,  Some("MathWallet".as_bytes().to_vec()))), 900000, Some(Vec::from_hex("6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c6574").unwrap())));

        let r = parse_deposit_outputs_impl::<Test>(&t9, &hot_addr).unwrap();
        assert_eq!(r, (Some((999,  None)), 900000, Some(Vec::from_hex("6a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e").unwrap())));
    });
}
