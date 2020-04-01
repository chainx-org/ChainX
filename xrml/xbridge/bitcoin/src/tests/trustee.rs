use super::*;

#[test]
pub fn test_check_trustee_entity() {
    let addr_ok_3 = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let public3 = Public::from_slice(&addr_ok_3).map_err(|_| "Invalid Public");
    assert_eq!(XBridgeOfBTC::check_trustee_entity(&addr_ok_3), public3);

    let addr_ok_2 = hex!("0211252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let public2 = Public::from_slice(&addr_ok_2).map_err(|_| "Invalid Public");
    assert_eq!(XBridgeOfBTC::check_trustee_entity(&addr_ok_2), public2);

    let addr_too_long =
        hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40cc");
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_too_long),
        Err("Invalid Public")
    );

    let addr_normal= hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae4011252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_normal),
        Err("not allow Normal Public for bitcoin now")
    );

    let addr_err_type = hex!("0411252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_err_type),
        Err("not Compressed Public(prefix not 2|3)")
    );

    let addr_zero = hex!("020000000000000000000000000000000000000000000000000000000000000000");
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_zero),
        Err("not Compressed Public(Zero32)")
    );

    let addr_ec_p = hex!("02fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f");
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_ec_p),
        Err("not Compressed Public(EC_P)")
    );

    let addr_ec_p_2 = hex!("02fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc3f");
    assert_eq!(
        XBridgeOfBTC::check_trustee_entity(&addr_ec_p_2),
        Err("not Compressed Public(EC_P)")
    );
}

#[test]
pub fn test_multi_address() {
    let pubkey1_bytes = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let pubkey2_bytes = hex!("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2");
    let pubkey3_bytes = hex!("023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d");

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

#[test]
fn test_create_multi_address() {
    //hot
    let pubkey1_bytes = hex!("03f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba");
    let pubkey2_bytes = hex!("0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd");
    let pubkey3_bytes = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let pubkey4_bytes = hex!("0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3");

    //cold
    let pubkey5_bytes = hex!("02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee");
    let pubkey6_bytes = hex!("03ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d70780");
    let pubkey7_bytes = hex!("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2");
    let pubkey8_bytes = hex!("020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f");

    let mut hot_keys = Vec::new();
    let mut cold_keys = Vec::new();
    hot_keys.push(Public::from_slice(&pubkey1_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey2_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey3_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey4_bytes).unwrap());

    cold_keys.push(Public::from_slice(&pubkey5_bytes).unwrap());
    cold_keys.push(Public::from_slice(&pubkey6_bytes).unwrap());
    cold_keys.push(Public::from_slice(&pubkey7_bytes).unwrap());
    cold_keys.push(Public::from_slice(&pubkey8_bytes).unwrap());
    //hot_keys.sort();

    with_externalities(&mut new_test_mainnet(), || {
        let hot_info = create_multi_address::<Test>(&hot_keys, 3).unwrap();
        let cold_info = create_multi_address::<Test>(&cold_keys, 3).unwrap();
        let real_hot_addr = "39eBWF3miGWb4CPiHw4MfsSwHcjtGq2pYL".as_bytes().to_vec();
        let real_cold_addr = "3AWmpzJ1kSF1cktFTDEb3qmLcdN8YydxA7".as_bytes().to_vec();
        assert_eq!(addr2vecu8(&hot_info.addr), real_hot_addr);
        assert_eq!(addr2vecu8(&cold_info.addr), real_cold_addr);

        let pks = [
            169, 20, 87, 55, 193, 151, 147, 67, 146, 12, 238, 164, 14, 124, 125, 104, 178, 100,
            176, 239, 250, 62, 135,
        ];
        let pk = hot_info.addr.hash.as_bytes();
        let mut pubkeys = Vec::new();
        pubkeys.push(Opcode::OP_HASH160 as u8);
        pubkeys.push(Opcode::OP_PUSHBYTES_20 as u8);
        pubkeys.extend_from_slice(pk);
        pubkeys.push(Opcode::OP_EQUAL as u8);
        assert_eq!(pubkeys, pks);
    });
}

#[test]
fn test_verify_signed() {
    use crate::tx::validator::parse_and_check_signed_tx_impl;

    let full_sig_tx = "010000000317840b38d466580696e9cb065c7a7aa55cb58cd5eb2526a10c3a30cc06d4b50a05000000fdfd0000483045022100dabbf878df8cacb23c08a8b5414cd64392a3f84777db4c01d8eec1e06d2e03fb0220502bd6e3960b68452699a40debfd92ac02e45d1526a2b570f5b28abdb496706401473044022047c58c3ad586d93f4b4caf65230a21e0ff70475b66affb8d4f92e916e6f6f664022029231b30472a949648dd99585ccbb169ccc2c007ad5387f580d41affdc8b37b6014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff853c87b1ecb4e881f323fec5314cb8623ca15de1341694e8352f99c434e7046a02000000fdfe0000483045022100b1b2233f70434f4079c1a8be1be5843b4dfe1edea30a3533aa94781af9984b2e02201ef78527ced51c7b122568666b9499d9cd2d4c3e704f5a54ebe433489c91b20101483045022100bde660b2f6f3c6fa512794377564289cbfcbeab6ecba1fe3b0b1531ebaa7d00a02207ea5435312280e0b502de715a6cbff7de866ba508a5fe8a644b88540ed471aee014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff442214a2d5a31195d6849005699892f60d48d89bca15bdb4ad6349c083e9936202000000fdfd000047304402205960c277575a7d2bb719211fe9cee0dd398c5a64d3a258fb0f877ae176dd11af02206cc0be53b1d5ea59477f9d2103ce06b61608561ac466c72235e86b26fe45734d01483045022100dcbd79d6f2d9504e2ea1578b7fdc9f98dadc018708acb4b87bd8b154312edfaa022043197a5b72219dc9603a81146a65c724a09022229ada2e3101a002dbd834b591014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff0340ebd201000000001976a9148e2fbed4fc7481a9a51f2bfe204301a122473f2f88ac406fdf25000000001976a914ede61104eddc07594f0c0cf43fecb9675353d16288ac91a3f6070000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000".into();
    let script = "522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253ae".into();
    let r = parse_and_check_signed_tx_impl(&full_sig_tx, script);
    assert_eq!(r, Ok(2))
}
