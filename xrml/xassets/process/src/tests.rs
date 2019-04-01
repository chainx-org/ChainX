// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
use support::{assert_err, assert_ok};

#[test]
fn test_check_btc_addr() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XRecords::deposit(&1, &b"BTC".to_vec(), 1000));

        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            XProcess::withdraw(
                origin,
                b"BTC".to_vec(),
                100,
                b"sdfds".to_vec(),
                b"".to_vec()
            ),
            "verify btc addr err"
        );

        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(XProcess::withdraw(
            origin,
            b"BTC".to_vec(),
            100,
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
            b"".to_vec()
        ));

        assert_eq!(XAssets::free_balance(&1, &b"BTC".to_vec()), 900);

        let nums = XRecords::withdrawal_application_numbers(Chain::Bitcoin, 10).unwrap();
        for n in nums {
            assert_ok!(XRecords::withdrawal_finish(n, true));
        }
        assert_eq!(XAssets::all_type_balance_of(&1, &b"BTC".to_vec()), 900)
    })
}

#[test]
fn test_check_btc_addr2() {
    with_externalities(&mut new_test_ext(), || {
        let r = XProcess::verify_addr(
            &XBitCoin::TOKEN.to_vec(),
            b"2N8tR484JD32i1DY2FnRPLwBVaNuXSfzoAv",
            b"",
        );
        assert_eq!(r, Ok(()));

        let r = XProcess::verify_addr(
            &XBitCoin::TOKEN.to_vec(),
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b",
            b"",
        );
        assert_eq!(r, Ok(()));
    })
}

#[test]
fn test_check_min_withdrawal() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAssets::issue(&b"BTC".to_vec(), &1, 1000));

        // less
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            XProcess::withdraw(
                origin,
                b"BTC".to_vec(),
                5,
                b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
                b"".to_vec()
            ),
            "withdrawal value should larger than requirement"
        );
        // equal
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            XProcess::withdraw(
                origin,
                b"BTC".to_vec(),
                10,
                b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
                b"".to_vec()
            ),
            "withdrawal value should larger than requirement"
        );
        // success
        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(XProcess::withdraw(
            origin,
            b"BTC".to_vec(),
            11,
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
            b"".to_vec()
        ));
    });
}

#[test]
fn test_check_blacklist() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAssets::issue(&b"BTC".to_vec(), &1, 1000));

        // success
        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(XProcess::withdraw(
            origin,
            b"BTC".to_vec(),
            11,
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
            b"".to_vec()
        ));

        // failed
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            XProcess::withdraw(
                origin,
                b"XDOT".to_vec(),
                11,
                b"xxx".to_vec(),
                b"xxx".to_vec()
            ),
            "this token is in blacklist"
        );

        // failed
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            XProcess::withdraw(
                origin,
                b"PCX".to_vec(),
                11,
                b"xxx".to_vec(),
                b"xxx".to_vec()
            ),
            "Can\'t withdraw the asset on ChainX"
        );
    });
}
