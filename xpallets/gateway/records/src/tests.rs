// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub use super::mock::*;
use super::*;

use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

#[test]
fn test_normal() {
    ExtBuilder::default().build_and_execute(|| {
        // deposit
        assert_ok!(XRecords::deposit(&ALICE, X_BTC, 100));
        assert_eq!(XAssets::usable_balance(&ALICE, &X_BTC), 100 + 100);

        // withdraw
        assert_ok!(XRecords::withdraw(
            &ALICE,
            X_BTC,
            50,
            b"addr".to_vec(),
            b"ext".to_vec().into()
        ));

        let numbers = XRecords::withdrawals_list_by_chain(Chain::Bitcoin)
            .into_iter()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(numbers.len(), 1);

        assert_ok!(XRecords::process_withdrawals(&numbers, Chain::Bitcoin));
        for i in numbers {
            assert_ok!(XRecords::finish_withdrawal(i, None));
        }
        assert_eq!(XAssets::usable_balance(&ALICE, &X_BTC), 50 + 100);
    })
}

#[test]
fn test_normal2() {
    ExtBuilder::default().build_and_execute(|| {
        // deposit
        assert_ok!(XRecords::deposit(&ALICE, X_BTC, 100));
        assert_eq!(XAssets::usable_balance(&ALICE, &X_BTC), 100 + 100);
        assert_ok!(XRecords::deposit(&ALICE, X_ETH, 500));
        assert_eq!(XAssets::usable_balance(&ALICE, &X_ETH), 500 + 100);

        // withdraw
        assert_ok!(XRecords::withdraw(
            &ALICE,
            X_BTC,
            50,
            b"addr".to_vec(),
            b"ext".to_vec().into()
        ));
        // withdrawal twice at once
        assert_ok!(XRecords::withdraw(
            &ALICE,
            X_ETH,
            100,
            b"addr".to_vec(),
            b"ext".to_vec().into()
        ));
        assert_ok!(XRecords::withdraw(
            &ALICE,
            X_ETH,
            50,
            b"addr".to_vec(),
            b"ext".to_vec().into()
        ));

        let numbers1 = XRecords::withdrawals_list_by_chain(Chain::Bitcoin)
            .into_iter()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(numbers1.len(), 1);

        let numbers2 = XRecords::withdrawals_list_by_chain(Chain::Ethereum)
            .into_iter()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(numbers2.len(), 2);

        let mut wrong_numbers = numbers1.clone();
        wrong_numbers.extend_from_slice(&numbers2);

        assert_noop!(
            XRecords::process_withdrawals(&wrong_numbers, Chain::Bitcoin),
            XRecordsErr::UnexpectedChain
        );
        assert_ok!(XRecords::process_withdrawals(&numbers1, Chain::Bitcoin));
        assert_ok!(XRecords::process_withdrawals(&numbers2, Chain::Ethereum));

        assert_ok!(XRecords::finish_withdrawals(
            &numbers1,
            Some(Chain::Bitcoin)
        ));
        assert_ok!(XRecords::finish_withdrawals(
            &numbers2,
            Some(Chain::Ethereum)
        ));

        assert_eq!(XAssets::usable_balance(&ALICE, &X_BTC), 50 + 100);
        assert_eq!(
            XAssets::usable_balance(&ALICE, &X_ETH),
            500 + 100 - 50 - 100
        );
    })
}

#[test]
fn test_withdrawal_more_then_usable() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(XRecords::deposit(&ALICE, X_BTC, 10));

        assert_noop!(
            XRecords::withdraw(
                &ALICE,
                X_BTC,
                100 + 50,
                b"addr".to_vec(),
                b"ext".to_vec().into()
            ),
            xpallet_assets::Error::<Test>::InsufficientBalance
        );
    })
}

#[test]
fn test_withdrawal_force_set_state() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(XRecords::deposit(&ALICE, X_BTC, 10));
        // applying
        assert_ok!(XRecords::withdraw(
            &ALICE,
            X_BTC,
            10,
            b"addr".to_vec(),
            b"ext".to_vec().into()
        ));
        assert_eq!(XAssets::usable_balance(&ALICE, &X_BTC), 100);
        // ignore processing state, force release locked balance
        assert_ok!(XRecords::set_withdrawal_state(
            RawOrigin::Root.into(),
            0,
            WithdrawalState::RootCancel
        ));
        assert_eq!(XAssets::usable_balance(&ALICE, &X_BTC), 100 + 10);
        // change to processing
        assert_ok!(XRecords::withdraw(
            &ALICE,
            X_BTC,
            10,
            b"addr".to_vec(),
            b"ext".to_vec().into()
        ));
        assert_ok!(XRecords::set_withdrawal_state(
            RawOrigin::Root.into(),
            1,
            WithdrawalState::Processing
        ));
        // reject revoke for a processing state
        assert_noop!(
            XRecords::cancel_withdrawal(1, &ALICE),
            XRecordsErr::NotApplyingState
        );
        // force change to applying
        assert_ok!(XRecords::set_withdrawal_state(
            RawOrigin::Root.into(),
            1,
            WithdrawalState::Applying
        ));
        assert_eq!(XRecords::state_of(1), Some(WithdrawalState::Applying));
    })
}

#[test]
fn test_withdrawal_chainx() {
    ExtBuilder::default().build_and_execute(|| {
        assert_noop!(
            XRecords::deposit(&ALICE, ChainXAssetId::get(), 10),
            xpallet_assets::Error::<Test>::DenyNativeAsset
        );

        assert_noop!(
            XRecords::withdraw(
                &ALICE,
                ChainXAssetId::get(),
                50,
                b"addr".to_vec(),
                b"ext".to_vec().into()
            ),
            xpallet_assets::Error::<Test>::DenyNativeAsset
        );
    })
}
