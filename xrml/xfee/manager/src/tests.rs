// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
use support::assert_ok;

#[test]
fn test_fee() {
    with_externalities(&mut new_test_ext(), || {
        xsystem::BlockProducer::<Test>::put(99);

        assert_ok!(XFeeManager::make_payment(&1, 10, 10, 1));
        // base fee = 10, bytes fee = 1
        let fee = 10 * 10 + 1 * 10;
        assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
        // block producer
        assert_eq!(XAssets::pcx_free_balance(&99), fee / 10);
        // jackpot account
        assert_eq!(XAssets::pcx_free_balance(&1000), fee * 9 / 10);
        // death account
        assert_eq!(XAssets::pcx_free_balance(&0), 0);
    });
}

#[test]
fn test_fee_no_blockproducer() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XFeeManager::make_payment(&1, 10, 10, 1));
        // base fee = 10, bytes fee = 1
        let fee = 10 * 10 + 1 * 10;
        assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
        // block producer
        // death account
        assert_eq!(XAssets::pcx_free_balance(&0), fee);
    });
}

#[test]
fn test_fee_not_divisible() {
    with_externalities(&mut new_test_ext(), || {
        xsystem::BlockProducer::<Test>::put(99);
        assert_ok!(XFeeManager::make_payment(&1, 11, 10, 1));
        // base fee = 10, bytes fee = 1
        let fee = 10 * 10 + 1 * 11; // 111
        assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
        // block producer
        assert_eq!(XAssets::pcx_free_balance(&99), fee / 10); // 11
                                                              // jackpot account
        assert_eq!(XAssets::pcx_free_balance(&1000), fee * 9 / 10 + 1); // 111 * 9 / 10 = 99 + 1 = 100
    });
}
