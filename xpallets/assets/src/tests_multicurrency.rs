// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use crate::mock::*;
use crate::*;

use frame_support::{assert_noop, assert_ok, traits::LockIdentifier};

use frame_support::traits::BalanceStatus;
use orml_traits::currency::{
    MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
};

pub const ID_1: LockIdentifier = *b"1       ";
pub const ID_2: LockIdentifier = *b"2       ";

#[test]
fn set_lock_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        XAssets::set_lock(ID_1, X_BTC, &ALICE, 10);
        assert_eq!(XAssets::locked_balance(&ALICE, &X_BTC), 10);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 1);
        XAssets::set_lock(ID_1, X_BTC, &ALICE, 50);
        assert_eq!(XAssets::locked_balance(&ALICE, &X_BTC), 50);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 1);
        XAssets::set_lock(ID_2, X_BTC, &ALICE, 60);
        assert_eq!(XAssets::locked_balance(&ALICE, &X_BTC), 60);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 2);
    });
}

#[test]
fn extend_lock_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        XAssets::set_lock(ID_1, X_BTC, &ALICE, 10);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 1);
        assert_eq!(XAssets::locked_balance(&ALICE, &X_BTC), 10);
        XAssets::extend_lock(ID_1, X_BTC, &ALICE, 20);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 1);
        assert_eq!(XAssets::locked_balance(&ALICE, &X_BTC), 20);
        XAssets::extend_lock(ID_2, X_BTC, &ALICE, 10);
        XAssets::extend_lock(ID_1, X_BTC, &ALICE, 20);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 2);
    });
}

#[test]
fn remove_lock_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        XAssets::set_lock(ID_1, X_BTC, &ALICE, 10);
        XAssets::set_lock(ID_2, X_BTC, &ALICE, 20);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 2);
        XAssets::remove_lock(ID_2, X_BTC, &ALICE);
        assert_eq!(XAssets::locks(ALICE, X_BTC).len(), 1);
    });
}

#[test]
fn frozen_can_limit_liquidity() {
    ExtBuilder::default().build_and_execute(|| {
        XAssets::set_lock(ID_1, X_BTC, &ALICE, 90);
        assert_noop!(
            <XAssets as MultiCurrency<_>>::transfer(X_BTC, &ALICE, &BOB, 11),
            XAssetsErr::LiquidityRestrictions,
        );
        XAssets::set_lock(ID_1, X_BTC, &ALICE, 10);
        assert_ok!(<XAssets as MultiCurrency<_>>::transfer(
            X_BTC, &ALICE, &BOB, 11
        ),);
    });
}

#[test]
fn can_reserve_is_correct() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(XAssets::can_reserve(X_BTC, &ALICE, 0), true);
        assert_eq!(XAssets::can_reserve(X_BTC, &ALICE, 101), false);
        assert_eq!(XAssets::can_reserve(X_BTC, &ALICE, 100), true);
    });
}

#[test]
fn reserve_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_noop!(
            XAssets::reserve(X_BTC, &ALICE, 101),
            XAssetsErr::InsufficientBalance,
        );
        assert_ok!(XAssets::reserve(X_BTC, &ALICE, 0));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 100);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 0);
        assert_eq!(XAssets::total_balance(X_BTC, &ALICE), 100);
        assert_ok!(XAssets::reserve(X_BTC, &ALICE, 50));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::total_balance(X_BTC, &ALICE), 100);
    });
}

#[test]
fn unreserve_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 100);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 0);
        assert_eq!(XAssets::unreserve(X_BTC, &ALICE, 0), 0);
        assert_eq!(XAssets::unreserve(X_BTC, &ALICE, 50), 50);
        assert_ok!(XAssets::reserve(X_BTC, &ALICE, 30));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 70);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 30);
        assert_eq!(XAssets::unreserve(X_BTC, &ALICE, 15), 0);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 85);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 15);
        assert_eq!(XAssets::unreserve(X_BTC, &ALICE, 30), 15);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 100);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 0);
    });
}

#[test]
fn slash_reserved_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(XAssets::reserve(X_BTC, &ALICE, 50));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::total_issuance(&X_BTC), 1000);
        assert_eq!(XAssets::slash_reserved(X_BTC, &ALICE, 0), 0);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::total_issuance(&X_BTC), 1000);
        assert_eq!(XAssets::slash_reserved(X_BTC, &ALICE, 100), 50);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 0);
        assert_eq!(XAssets::total_issuance(&X_BTC), 1000); // we do not slash balance, slashed balance auto send to
    });
}

#[test]
fn repatriate_reserved_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(XAssets::slash(X_BTC, &BOB, 100), 0);

        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 100);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 0);
        assert_eq!(
            XAssets::repatriate_reserved(X_BTC, &ALICE, &ALICE, 0, BalanceStatus::Free),
            Ok(0)
        );
        assert_eq!(
            XAssets::repatriate_reserved(X_BTC, &ALICE, &ALICE, 50, BalanceStatus::Free),
            Ok(50)
        );
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 100);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 0);

        assert_eq!(XAssets::free_balance(X_BTC, &BOB), 100);
        assert_eq!(XAssets::reserved_balance(X_BTC, &BOB), 0);
        assert_ok!(XAssets::reserve(X_BTC, &BOB, 50));
        assert_eq!(XAssets::free_balance(X_BTC, &BOB), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &BOB), 50);
        assert_eq!(
            XAssets::repatriate_reserved(X_BTC, &BOB, &BOB, 60, BalanceStatus::Reserved),
            Ok(10)
        );
        assert_eq!(XAssets::free_balance(X_BTC, &BOB), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &BOB), 50);

        assert_eq!(
            XAssets::repatriate_reserved(X_BTC, &BOB, &ALICE, 30, BalanceStatus::Reserved),
            Ok(0)
        );
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 100);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 30);
        assert_eq!(XAssets::free_balance(X_BTC, &BOB), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &BOB), 20);

        assert_eq!(
            XAssets::repatriate_reserved(X_BTC, &BOB, &ALICE, 30, BalanceStatus::Free),
            Ok(10)
        );
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 120);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 30);
        assert_eq!(XAssets::free_balance(X_BTC, &BOB), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &BOB), 0);
    });
}

#[test]
fn slash_draw_reserved_correct() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(XAssets::reserve(X_BTC, &ALICE, 50));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::total_issuance(&X_BTC), 200 + 800);

        assert_eq!(XAssets::slash(X_BTC, &ALICE, 80), 0);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 0);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 20);
        assert_eq!(XAssets::total_issuance(&X_BTC), 200 + 800);

        assert_eq!(XAssets::slash(X_BTC, &ALICE, 50), 30);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 0);
        assert_eq!(XAssets::reserved_balance(X_BTC, &ALICE), 0);
        assert_eq!(XAssets::total_issuance(&X_BTC), 200 + 800);
    });
}

#[test]
fn transfer_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(XAssets::slash(X_BTC, &BOB, 100), 0);
        System::set_block_number(2);

        assert_ok!(<XAssets as MultiCurrency<_>>::transfer(
            X_BTC, &ALICE, &BOB, 50
        ));
        // assert_ok!(XAssets::transfer(Some(ALICE).into(), BOB, X_BTC, 50));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::free_balance(X_BTC, &BOB), 150);
        assert_eq!(XAssets::total_issuance(&X_BTC), 200 + 800);

        let transferred_event = MetaEvent::assets(RawEvent::Moved(
            X_BTC,
            ALICE,
            AssetType::Usable,
            BOB,
            AssetType::Usable,
            50,
        ));
        assert!(System::events()
            .iter()
            .any(|record| record.event == transferred_event));

        assert_noop!(
            <XAssets as MultiCurrency<_>>::transfer(X_BTC, &ALICE, &BOB, 60),
            XAssetsErr::InsufficientBalance,
        );
    });
}

#[test]
fn deposit_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(XAssets::deposit(X_BTC, &ALICE, 100));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 200);
        assert_eq!(XAssets::total_issuance(&X_BTC), 300 + 800);

        assert_noop!(
            XAssets::deposit(X_BTC, &ALICE, Balance::max_value()),
            XAssetsErr::Overflow,
        );
    });
}

#[test]
fn withdraw_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(XAssets::withdraw(X_BTC, &ALICE, 50));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::total_issuance(&X_BTC), 1000 - 50);

        assert_noop!(
            XAssets::withdraw(X_BTC, &ALICE, 60),
            XAssetsErr::InsufficientBalance
        );
    });
}

#[test]
fn slash_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        // slashed_amount < amount
        assert_eq!(XAssets::slash(X_BTC, &ALICE, 50), 0);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 50);
        assert_eq!(XAssets::total_issuance(&X_BTC), 1000);

        // slashed_amount == amount
        assert_eq!(XAssets::slash(X_BTC, &ALICE, 51), 1);
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 0);
        assert_eq!(XAssets::total_issuance(&X_BTC), 1000);
    });
}

#[test]
fn update_balance_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(XAssets::slash(X_BTC, &BOB, 100), 0);

        assert_ok!(XAssets::update_balance(X_BTC, &ALICE, 50));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 150);
        assert_eq!(XAssets::total_issuance(&X_BTC), 250 + 800);

        assert_ok!(XAssets::update_balance(X_BTC, &BOB, -50));
        assert_eq!(XAssets::free_balance(X_BTC, &BOB), 50);
        assert_eq!(XAssets::total_issuance(&X_BTC), 200 + 800);

        assert_noop!(
            XAssets::update_balance(X_BTC, &BOB, -60),
            XAssetsErr::InsufficientBalance
        );
    });
}

#[test]
fn ensure_can_withdraw_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_noop!(
            XAssets::ensure_can_withdraw(X_BTC, &ALICE, 101),
            XAssetsErr::InsufficientBalance
        );

        assert_ok!(XAssets::ensure_can_withdraw(X_BTC, &ALICE, 1));
        assert_eq!(XAssets::free_balance(X_BTC, &ALICE), 100);
    });
}

#[test]
fn no_op_if_amount_is_zero() {
    let btc_assets = btc();
    let assets = vec![(btc_assets.0, btc_assets.1, btc_assets.2, true, true)];
    ExtBuilder::default()
        .build(assets, Default::default())
        .execute_with(|| {
            assert_ok!(XAssets::ensure_can_withdraw(X_BTC, &ALICE, 0));
            assert_ok!(<XAssets as MultiCurrency<_>>::transfer(
                X_BTC, &ALICE, &BOB, 0
            ));
            assert_ok!(<XAssets as MultiCurrency<_>>::transfer(
                X_BTC, &ALICE, &ALICE, 0
            ));
            assert_ok!(XAssets::deposit(X_BTC, &ALICE, 0));
            assert_ok!(XAssets::withdraw(X_BTC, &ALICE, 0));
            assert_eq!(XAssets::slash(X_BTC, &ALICE, 0), 0);
            assert_eq!(XAssets::slash(X_BTC, &ALICE, 1), 1);
            assert_ok!(XAssets::update_balance(X_BTC, &ALICE, 0));
        });
}
