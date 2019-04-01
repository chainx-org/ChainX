// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
use support::{assert_err, assert_ok};

#[test]
fn test_normal() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();

        // deposit
        assert_ok!(XRecords::deposit(&a, &btc_token, 100));
        assert_eq!(XAssets::free_balance(&a, &btc_token), 100);

        // withdraw
        assert_ok!(XRecords::withdrawal(
            &a,
            &btc_token,
            50,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));

        let numbers = XRecords::withdrawal_application_numbers(Chain::Bitcoin, 10).unwrap();
        assert_eq!(numbers.len(), 1);

        for i in numbers {
            assert_ok!(XRecords::withdrawal_finish(i, true));
        }
        assert_eq!(XAssets::free_balance(&a, &btc_token), 50);
    })
}

#[test]
fn test_normal2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        let eth_token = b"ETH".to_vec();

        // deposit
        assert_ok!(XRecords::deposit(&a, &btc_token, 100));
        assert_eq!(XAssets::free_balance(&a, &btc_token), 100);
        assert_ok!(XRecords::deposit(&a, &eth_token, 500));
        assert_eq!(XAssets::free_balance(&a, &eth_token), 500);

        // withdraw
        assert_ok!(XRecords::withdrawal(
            &a,
            &btc_token,
            50,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));
        // withdrawal twice at once
        assert_ok!(XRecords::withdrawal(
            &a,
            &eth_token,
            100,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));
        assert_ok!(XRecords::withdrawal(
            &a,
            &eth_token,
            50,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));

        let mut numbers1 = XRecords::withdrawal_application_numbers(Chain::Bitcoin, 10).unwrap();
        assert_eq!(numbers1.len(), 1);

        let numbers2 = XRecords::withdrawal_application_numbers(Chain::Ethereum, 10).unwrap();
        assert_eq!(numbers2.len(), 2);

        numbers1.extend(numbers2);

        for i in numbers1 {
            assert_ok!(XRecords::withdrawal_finish(i, true));
        }
        assert_eq!(XAssets::free_balance(&a, &btc_token), 50);
        assert_eq!(XAssets::free_balance(&a, &eth_token), 500 - 50 - 100);
    })
}

#[test]
fn test_withdrawal_larger() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        assert_ok!(XRecords::deposit(&a, &btc_token, 10));

        assert_err!(
            XRecords::withdrawal(&a, &btc_token, 50, b"addr".to_vec(), b"ext".to_vec()),
            "free balance not enough for this account"
        );
    })
}

#[test]
fn test_withdrawal_chainx() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let chainx_token = XAssets::TOKEN.to_vec();
        assert_err!(
            XRecords::deposit(&a, &chainx_token, 10),
            "can\'t deposit/withdrawal chainx token"
        );

        assert_err!(
            XRecords::withdrawal(&a, &chainx_token, 50, b"addr".to_vec(), b"ext".to_vec()),
            "can\'t deposit/withdrawal chainx token"
        );
    })
}

#[test]
fn test_withdrawal_first() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        assert_err!(
            XRecords::withdrawal(&a, &btc_token, 50, vec![], vec![]),
            "free balance not enough for this account"
        );
    })
}
