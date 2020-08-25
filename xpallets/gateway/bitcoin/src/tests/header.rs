use super::*;

#[test]
fn test_genesis() {
    ExtBuilder::default().build_and_execute(|| {
        let (header, num) = XGatewayBitcoin::genesis_info();
        assert_eq!(
            format!("{:?}", reverse_h256(header.hash())),
            "0x0000000000000000001721f58deb88b0710295a02551f0dde1e2e231a15f1882"
        );
        assert_eq!(num, 576576);

        let index = XGatewayBitcoin::best_index();
        assert_eq!(
            index,
            BtcHeaderIndex {
                hash: header.hash(),
                height: 576576
            }
        );
    })
}

#[test]
fn test_insert_headers() {
    let (base_height, c1, _) = generate_blocks();
    ExtBuilder::default()
        .build_mock(
            (c1.get(0).unwrap().clone(), base_height),
            BtcNetwork::Mainnet,
        )
        .execute_with(|| {
            assert_noop!(
                XGatewayBitcoin::apply_push_header(c1.get(0).unwrap().clone()),
                XGatewayBitcoinErr::ExistedHeader
            );

            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(1).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(2).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(3).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(4).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(5).unwrap().clone()
            ));

            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, c1.get(5).unwrap().hash());

            should_in_mainchain(&c1, true);
        })
}

fn should_in_mainchain(headers: &[BtcHeader], expect: bool) {
    for header in headers.iter() {
        assert_eq!(
            XGatewayBitcoin::main_chain(&header.hash()).is_some(),
            expect
        );
    }
}

#[test]
fn test_insert_forked_headers_from_genesis_height() {
    // e.g.
    // b1
    // b --- b --- b --- b
    // |---- b --- b
    let (base_height, c1, forked) = generate_blocks();
    ExtBuilder::default()
        .build_mock(
            (c1.get(1).unwrap().clone(), base_height + 1),
            BtcNetwork::Mainnet,
        )
        .execute_with(|| {
            // note: confirm block is 4
            assert_noop!(
                XGatewayBitcoin::apply_push_header(c1.get(1).unwrap().clone()),
                XGatewayBitcoinErr::ExistedHeader
            );

            // insert first
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(2).unwrap().clone()
            ));
            // best index is normal block 2
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, c1.get(2).unwrap().hash());

            // insert forked first
            assert_ok!(XGatewayBitcoin::apply_push_header(
                forked.get(2).unwrap().clone()
            ));
            // contains two block at height 2, but best index still normal
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, c1.get(2).unwrap().hash());
            should_in_mainchain(&c1[1..3], true);

            assert_ok!(XGatewayBitcoin::apply_push_header(
                forked.get(3).unwrap().clone()
            ));
            // forked block overtake than normal, change current best to forked 3
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, forked.get(3).unwrap().hash());
            assert_eq!(XGatewayBitcoin::confirmed_index(), None);
            should_in_mainchain(&c1[2..3], false);
            should_in_mainchain(&forked[1..4], true);

            // start insert normal
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(3).unwrap().clone()
            ));
            // because forked 3 insert before, so that even receive normal 3, best still forked 3
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, forked.get(3).unwrap().hash());

            // switch forked to normal chain
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(4).unwrap().clone()
            ));
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, c1.get(4).unwrap().hash());
            // confirm set to 1
            let confirmed_index = XGatewayBitcoin::confirmed_index().unwrap();
            assert_eq!(confirmed_index.hash, c1.get(1).unwrap().hash());

            // move confirmed in normal chain
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(5).unwrap().clone()
            ));
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, c1.get(5).unwrap().hash());
            // confirm set to 2
            let confirmed_index = XGatewayBitcoin::confirmed_index().unwrap();
            assert_eq!(confirmed_index.hash, c1.get(2).unwrap().hash());

            should_in_mainchain(&c1[1..6], true);
            should_in_mainchain(&forked[2..4], false);
            println!("current confirmed height:{:?}", confirmed_index.height);

            // add a forked block exceed confirmed height, this forked block would mark as AncientFork,
            // but now this forked block is less then best, so do not do confirmed check
            // and on the other hand, confirmed block for this forked block is before current normal
            assert_ok!(XGatewayBitcoin::apply_push_header(
                forked.get(4).unwrap().clone()
            ));
            // but when add a more forked block, would try to move confirmed
            assert_noop!(
                XGatewayBitcoin::apply_push_header(forked.get(5).unwrap().clone()),
                XGatewayBitcoinErr::AncientFork,
            );
        })
}

#[test]
fn test_insert_forked_headers() {
    // e.g.
    // b0
    // b --- b --- b --- b --- b
    //       |---- b --- b
    let (base_height, c1, forked) = generate_blocks();
    ExtBuilder::default()
        .build_mock(
            (c1.get(0).unwrap().clone(), base_height),
            BtcNetwork::Mainnet,
        )
        .execute_with(|| {
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(1).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(2).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(3).unwrap().clone()
            ));
            should_in_mainchain(&c1[0..4], true);
            // now confirmed would set
            let confirmed_index = XGatewayBitcoin::confirmed_index().unwrap();
            assert_eq!(confirmed_index.hash, c1.get(0).unwrap().hash());

            // insert forked
            assert_ok!(XGatewayBitcoin::apply_push_header(
                forked.get(2).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                forked.get(3).unwrap().clone()
            ));
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, c1.get(3).unwrap().hash());
            let confirmed_index = XGatewayBitcoin::confirmed_index().unwrap();
            assert_eq!(confirmed_index.hash, c1.get(0).unwrap().hash());
            should_in_mainchain(&c1[0..4], true);
            should_in_mainchain(&forked[2..4], false);

            // insert forked, switch chain, but confirm b1, b1 is also the parent for normal chain
            assert_ok!(XGatewayBitcoin::apply_push_header(
                forked.get(4).unwrap().clone()
            ));
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, forked.get(4).unwrap().hash());
            let confirmed_index = XGatewayBitcoin::confirmed_index().unwrap();
            assert_eq!(confirmed_index.hash, c1.get(1).unwrap().hash());
            should_in_mainchain(&c1[2..4], false);
            should_in_mainchain(&forked[1..5], true);

            // b1 still on normal chain, so we could switch to normal and reset main chain
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(4).unwrap().clone()
            ));
            assert_ok!(XGatewayBitcoin::apply_push_header(
                c1.get(5).unwrap().clone()
            ));
            let best_index = XGatewayBitcoin::best_index();
            assert_eq!(best_index.hash, c1.get(5).unwrap().hash());
            let confirmed_index = XGatewayBitcoin::confirmed_index().unwrap();
            assert_eq!(confirmed_index.hash, c1.get(2).unwrap().hash());
            should_in_mainchain(&c1[0..6], true);
            should_in_mainchain(&forked[2..5], false);
        });
}

/*
#[test]
fn test_call() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        let origin = frame_system::RawOrigin::Signed(99).into();
        let v = btc_ser::serialize(c1.get(1).unwrap());
        let v = v.take();
        assert_ok!(XGatewayBitcoin::push_header(origin, v));
    })
}

#[test]
fn test_genesis2() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        assert_noop!(
            XGatewayBitcoin::apply_push_header(c1.get(0).unwrap().clone()),
            "Block parent is unknown"
        );
        assert_ok!(XGatewayBitcoin::apply_push_header(c1.get(1).unwrap().clone()));
        assert_ok!(XGatewayBitcoin::apply_push_header(c1.get(2).unwrap().clone()));
        assert_ok!(XGatewayBitcoin::apply_push_header(c1.get(3).unwrap().clone()));
    })
}

#[test]
fn test_changebit() {
    with_externalities(&mut new_test_ext(), || {
        let b1 = BtcHeader {
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

        let _b2 = BtcHeader {
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
