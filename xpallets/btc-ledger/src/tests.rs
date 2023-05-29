// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use crate::mock::*;

use frame_support::{
    assert_err, assert_noop, assert_ok,
    traits::{
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        WithdrawReasons,
    },
};
use frame_system::RawOrigin;
use sp_core::crypto::AccountId32;
use sp_runtime::{traits::BadOrigin, ArithmeticError};

#[test]
fn btc_ledger_account_id() {
    let account_id = BtcLedger::account_id();
    let expect = "5EYCAe5iijNKP1cK7TuhPY6Sa5FFnmuyrGtjJmMTQWwJ75Dg";

    assert_eq!(format!("{}", account_id), expect)
}

#[test]
fn account_zero_balance_should_be_not_reaped() {
    new_test_ext().execute_with(|| {
        assert!(frame_system::Account::<Test>::contains_key(
            &AccountId32::from(ALICE)
        ));

        assert_eq!(BtcLedger::free_balance(AccountId32::from(ALICE)), 10);
        assert_ok!(<BtcLedger as Currency<_>>::transfer(
            &ALICE.into(),
            &BOB.into(),
            10,
            AllowDeath
        ));

        // Check that the account is not dead.
        assert!(frame_system::Account::<Test>::contains_key(
            &AccountId32::from(ALICE)
        ));
    });
}

#[test]
fn account_provider_consumer_sufficient() {
    new_test_ext().execute_with(|| {
        // SCENARIO: From existing account to existing account
        assert_eq!(System::providers(&ALICE.into()), 1);
        assert_eq!(System::consumers(&ALICE.into()), 0);
        assert_eq!(System::sufficients(&ALICE.into()), 0);
        assert_eq!(System::providers(&BOB.into()), 1);
        assert_eq!(System::consumers(&BOB.into()), 0);
        assert_eq!(System::sufficients(&BOB.into()), 0);

        assert!(System::account_exists(&ALICE.into()));
        assert!(System::account_exists(&BOB.into()));
        assert_ok!(<BtcLedger as Currency<_>>::transfer(
            &ALICE.into(),
            &BOB.into(),
            5,
            AllowDeath
        ));
        assert!(System::account_exists(&ALICE.into()));
        assert!(System::account_exists(&BOB.into()));
        assert_eq!(BtcLedger::free_balance(AccountId32::from(ALICE)), 5);
        assert_eq!(BtcLedger::free_balance(AccountId32::from(BOB)), 25);

        assert_eq!(System::providers(&BOB.into()), 1);
        assert_eq!(System::consumers(&BOB.into()), 0);
        assert_eq!(System::sufficients(&BOB.into()), 0);

        // SCENARIO: From existing account to nonexistent account
        assert_eq!(System::providers(&ALICE.into()), 1);
        assert_eq!(System::consumers(&ALICE.into()), 0);
        assert_eq!(System::sufficients(&ALICE.into()), 0);
        assert_eq!(System::providers(&CHARLIE.into()), 0);
        assert_eq!(System::consumers(&CHARLIE.into()), 0);
        assert_eq!(System::sufficients(&CHARLIE.into()), 0);

        assert!(!System::account_exists(&CHARLIE.into()));
        assert_ok!(<BtcLedger as Currency<_>>::transfer(
            &ALICE.into(),
            &CHARLIE.into(),
            5,
            AllowDeath
        ));
        assert!(System::account_exists(&CHARLIE.into()));
        assert_eq!(BtcLedger::free_balance(AccountId32::from(ALICE)), 0);
        assert_eq!(BtcLedger::free_balance(AccountId32::from(CHARLIE)), 5);

        assert_eq!(System::providers(&ALICE.into()), 1);
        assert_eq!(System::consumers(&ALICE.into()), 0);
        assert_eq!(System::sufficients(&ALICE.into()), 0);
        assert_eq!(System::providers(&CHARLIE.into()), 0);
        assert_eq!(System::consumers(&CHARLIE.into()), 0);
        assert_eq!(System::sufficients(&CHARLIE.into()), 1);
    });
}

#[test]
fn reward_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 10);
        assert_ok!(BtcLedger::deposit_into_existing(&ALICE.into(), 10).map(drop));
        System::assert_last_event(Event::BtcLedger(crate::Event::Deposit {
            who: ALICE.into(),
            amount: 10,
        }));
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 20);
        assert_eq!(btc_ledger::TotalIssuance::<Test>::get(), 40);
    });
}

#[test]
fn balance_works() {
    new_test_ext().execute_with(|| {
        let _ = BtcLedger::deposit_creating(&ALICE.into(), 30);
        System::assert_has_event(Event::BtcLedger(crate::Event::Deposit {
            who: ALICE.into(),
            amount: 30,
        }));
        assert_eq!(BtcLedger::free_balance(&AccountId32::from(ALICE)), 40);
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 40);
        assert_eq!(BtcLedger::free_balance(AccountId32::from(BOB)), 20);
        assert_eq!(BtcLedger::total_balance(&BOB.into()), 20);
    });
}

#[test]
fn balance_transfer_works() {
    new_test_ext().execute_with(|| {
        let _ = BtcLedger::deposit_creating(&ALICE.into(), 40);
        assert_ok!(BtcLedger::transfer(
            Some(ALICE.into()).into(),
            BOB.into(),
            20
        ));
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 30);
        assert_eq!(BtcLedger::total_balance(&BOB.into()), 40);
    });
}

#[test]
fn force_transfer_works() {
    new_test_ext().execute_with(|| {
        let _ = BtcLedger::deposit_creating(&ALICE.into(), 50);
        assert_noop!(
            BtcLedger::force_transfer(Some(BOB.into()).into(), ALICE.into(), BOB.into(), 50),
            BadOrigin,
        );

        assert_ok!(BtcLedger::force_transfer(
            RawOrigin::Root.into(),
            ALICE.into(),
            BOB.into(),
            50
        ));
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 10);
        assert_eq!(BtcLedger::total_balance(&BOB.into()), 70);
    });
}

#[test]
fn withdrawing_balance_should_work() {
    new_test_ext().execute_with(|| {
        let _ = BtcLedger::deposit_creating(&BOB.into(), 100);
        let _ = BtcLedger::withdraw(&BOB.into(), 20, WithdrawReasons::TRANSFER, AllowDeath);

        System::assert_last_event(Event::BtcLedger(crate::Event::Withdraw {
            who: BOB.into(),
            amount: 20,
        }));

        assert_eq!(BtcLedger::free_balance(AccountId32::from(BOB)), 100);
        assert_eq!(btc_ledger::TotalIssuance::<Test>::get(), 110);

        let _ = BtcLedger::withdraw(&ALICE.into(), 10, WithdrawReasons::TRANSFER, KeepAlive);

        System::assert_last_event(Event::BtcLedger(crate::Event::Withdraw {
            who: ALICE.into(),
            amount: 10,
        }));

        assert_eq!(BtcLedger::free_balance(AccountId32::from(BOB)), 100);
        assert_eq!(btc_ledger::TotalIssuance::<Test>::get(), 100);
    });
}

#[test]
fn transferring_too_high_value_should_not_panic() {
    new_test_ext().execute_with(|| {
        BtcLedger::make_free_balance_be(&ALICE.into(), u128::MAX);
        BtcLedger::make_free_balance_be(&BOB.into(), 1);

        assert_err!(
            BtcLedger::transfer(Some(ALICE.into()).into(), BOB.into(), u128::MAX),
            ArithmeticError::Overflow,
        );

        assert_eq!(BtcLedger::free_balance(AccountId32::from(ALICE)), u128::MAX);
        assert_eq!(BtcLedger::free_balance(AccountId32::from(BOB)), 1);
    });
}

#[test]
fn burn_must_work() {
    new_test_ext().execute_with(|| {
        let init_total_issuance = BtcLedger::total_issuance();
        let imbalance = BtcLedger::burn(10);
        assert_eq!(BtcLedger::total_issuance(), init_total_issuance - 10);
        drop(imbalance);
        assert_eq!(BtcLedger::total_issuance(), init_total_issuance);
    });
}

#[test]
#[should_panic = "duplicate balances in genesis."]
fn cannot_set_genesis_value_twice() {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    let _ = btc_ledger::GenesisConfig::<Test> {
        balances: vec![(ALICE.into(), 10), (BOB.into(), 20), (ALICE.into(), 15)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
}

#[test]
fn transfer_all_free_succeed() {
    new_test_ext().execute_with(|| {
        assert_ok!(BtcLedger::set_balance(Origin::root(), ALICE.into(), 100));
        assert_ok!(BtcLedger::transfer(
            Some(ALICE.into()).into(),
            BOB.into(),
            100
        ));
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 0);
        assert_eq!(BtcLedger::total_balance(&BOB.into()), 120);
    });
}

#[test]
fn transfer_all_works() {
    new_test_ext().execute_with(|| {
        // setup
        assert_ok!(BtcLedger::set_balance(Origin::root(), ALICE.into(), 200));
        assert_ok!(BtcLedger::set_balance(Origin::root(), BOB.into(), 0));
        // transfer all and allow death
        assert_ok!(BtcLedger::transfer(
            Some(ALICE.into()).into(),
            BOB.into(),
            200
        ));
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 0);
        assert_eq!(BtcLedger::total_balance(&BOB.into()), 200);

        // setup
        assert_ok!(BtcLedger::set_balance(Origin::root(), ALICE.into(), 200));
        assert_ok!(BtcLedger::set_balance(Origin::root(), BOB.into(), 0));
        // transfer all and keep alive
        assert_ok!(BtcLedger::transfer(
            Some(ALICE.into()).into(),
            BOB.into(),
            200
        ));
        assert_eq!(BtcLedger::total_balance(&ALICE.into()), 0);
        assert_eq!(BtcLedger::total_balance(&BOB.into()), 200);
    });
}

#[test]
fn set_balance_handles_total_issuance() {
    new_test_ext().execute_with(|| {
        let old_total_issuance = BtcLedger::total_issuance();
        assert_ok!(BtcLedger::set_balance(Origin::root(), CHARLIE.into(), 69));
        assert_eq!(BtcLedger::total_issuance(), old_total_issuance + 69);
        assert_eq!(BtcLedger::total_balance(&CHARLIE.into()), 69);
        assert_eq!(BtcLedger::free_balance(&CHARLIE.into()), 69);
    });
}
