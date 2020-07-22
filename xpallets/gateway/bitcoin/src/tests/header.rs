use super::*;

#[test]
fn test() {
    with_externalities(&mut new_test_ext(), || {
        use substrate_primitives::hexdisplay::HexDisplay;
        let r = <Headers<Test>>::key_for(&h256_from_rev_str(
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

//#[test]
//fn test_genesis() {
//    with_externalities(&mut new_test_mock_ext(), || {
//        let (header, num) = XBridgeOfBTC::genesis_info();
//        let _r = <GenesisInfo<Test>>::get();
//        assert_eq!(
//            format!("{:?}", reverse_h256(header.hash())),
//            "00000000000000fd9cea8b846895f507c63b005d20ac56e87d1cdf80effd5c0a"
//        );
//        assert_eq!(num, 1457525);
//
//        let best_hash = XBridgeOfBTC::best_index();
//        assert_eq!(best_hash, header.hash());
//    })
//}
/*
#[test]
fn test_normal() {
    with_externalities(&mut new_test_mainnet(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
//        assert_err!(
//            XBridgeOfBTC::apply_push_header(c1.get(0).unwrap().clone()),
//            "Can\'t find previous header"
//        );
        assert_ok!(XBridgeOfBTC::apply_push_header(c1.get(1).unwrap().clone()));
//        assert_ok!(XBridgeOfBTC::apply_push_header(c1.get(2).unwrap().clone()));

//        let best_hash = XBridgeOfBTC::best_index();
//        assert_eq!(best_hash, c1.get(2).unwrap().hash());
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
    with_externalities(&mut new_test_ext(), || {
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
    with_externalities(&mut new_test_ext(), || {
        let b1 = BTCHeader {
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

        let _b2 = BTCHeader {
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
*/
