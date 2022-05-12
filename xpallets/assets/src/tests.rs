// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::BTreeMap;

use frame_support::{assert_noop, assert_ok, traits::Currency};
use frame_system::{EventRecord, Phase};

use xp_protocol::X_BTC;

pub use super::mock::{ExtBuilder, Test};
use crate::{
    mock::{Balance, Event, Origin, System, XAssets, XAssetsErr},
    AssetBalance, AssetErr, AssetInfo, AssetRestrictions, AssetType, Chain, Config,
    TotalAssetBalance,
};

#[test]
fn test_genesis() {
    let abc_id = 100;
    let efd_id = 101;
    let abc_assets = (
        abc_id,
        AssetInfo::new::<Test>(
            b"ABC".to_vec(),
            b"ABC".to_vec(),
            Chain::Bitcoin,
            8,
            b"abc".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DESTROY_USABLE,
    );

    let efd_assets = (
        efd_id,
        AssetInfo::new::<Test>(
            b"EFD".to_vec(),
            b"EFD Token".to_vec(),
            Chain::Bitcoin,
            8,
            b"efd".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::TRANSFER | AssetRestrictions::DESTROY_USABLE,
    );

    let mut endowed = BTreeMap::new();
    let endowed_info = vec![(1, 100), (2, 200), (3, 300), (4, 400)];
    endowed.insert(abc_assets.0, endowed_info);

    let endowed_info = vec![(999, 1000)];
    endowed.insert(efd_assets.0, endowed_info);

    let assets = vec![
        (abc_assets.0, abc_assets.1, abc_assets.2, true, true),
        (efd_assets.0, efd_assets.1, efd_assets.2, true, false),
    ];

    ExtBuilder::default()
        .build(assets, endowed)
        .execute_with(|| {
            assert_eq!(XAssets::total_issuance(&abc_id), 100 + 200 + 300 + 400);
            assert_eq!(XAssets::total_issuance(&efd_id), 1000);
            assert_eq!(XAssets::usable_balance(&1, &abc_id), 100);
            assert_eq!(XAssets::usable_balance(&4, &abc_id), 400);
            assert_eq!(XAssets::usable_balance(&999, &efd_id), 1000);

            assert_noop!(
                XAssets::destroy_usable(&abc_id, &1, 10),
                XAssetsErr::ActionNotAllowed
            );
            assert_ok!(XAssets::transfer(Origin::signed(1), 999, abc_id, 50_u128,));
            assert_noop!(
                XAssets::transfer(Origin::signed(999), 1, efd_id, 50_u128,),
                XAssetsErr::ActionNotAllowed
            );
        });
}

#[test]
fn test_normal_case() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(XAssets::total_issuance(&X_BTC), 100 + 200 + 300 + 400);

        assert_ok!(XAssets::transfer(Origin::signed(1), 999, X_BTC, 50_u128,));
        assert_eq!(XAssets::usable_balance(&1, &X_BTC), 50);
        assert_eq!(XAssets::usable_balance(&999, &X_BTC), 50);

        assert_eq!(XAssets::total_issuance(&X_BTC), 100 + 200 + 300 + 400);

        assert_ok!(XAssets::move_balance(
            &X_BTC,
            &1,
            AssetType::Usable,
            &999,
            AssetType::ReservedWithdrawal,
            25
        ));
        assert_eq!(
            XAssets::total_asset_balance_of(&X_BTC, AssetType::Usable),
            1000 - 25
        );
        assert_eq!(
            XAssets::total_asset_balance_of(&X_BTC, AssetType::ReservedWithdrawal),
            25
        );

        assert_ok!(XAssets::destroy_reserved_withdrawal(&X_BTC, &999, 15));
        assert_eq!(
            XAssets::asset_typed_balance(&999, &X_BTC, AssetType::ReservedWithdrawal),
            10
        );
        assert_eq!(
            XAssets::total_asset_balance_of(&X_BTC, AssetType::ReservedWithdrawal),
            10
        );
        assert_eq!(XAssets::total_issuance(&X_BTC), 100 + 200 + 300 + 400 - 15);

        assert_ok!(XAssets::destroy_reserved_withdrawal(&X_BTC, &999, 10));
        assert_eq!(
            XAssets::total_asset_balance_of(&X_BTC, AssetType::ReservedWithdrawal),
            0
        );
        // make sure the item is removed in btree-map
        assert!(XAssets::total_asset_balance(&X_BTC)
            .get(&AssetType::ReservedWithdrawal)
            .is_none());
        assert!(XAssets::asset_balance(&999, &X_BTC)
            .get(&AssetType::ReservedWithdrawal)
            .is_none());
        assert_eq!(XAssets::total_issuance(&X_BTC), 100 + 200 + 300 + 400 - 25);
    })
}

#[test]
fn test_normal_issue_and_destroy() {
    ExtBuilder::default().build_and_execute(|| {
        let a: u64 = 1; // accountid
        let btc_id = X_BTC;

        // issue
        XAssets::issue(&btc_id, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 150);
        assert_eq!(XAssets::total_issuance(&btc_id), 1050);

        // reserve
        XAssets::move_balance(
            &btc_id,
            &a,
            AssetType::Usable,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();

        assert_eq!(
            XAssets::asset_balance_of(&a, &btc_id, AssetType::ReservedWithdrawal),
            25
        );
        assert_eq!(XAssets::usable_balance(&a, &btc_id), 125);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 150);

        // destroy
        XAssets::destroy_reserved_withdrawal(&btc_id, &a, 25).unwrap();
        assert_eq!(
            XAssets::asset_balance_of(&a, &btc_id, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::usable_balance(&a, &btc_id), 125);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 125);
        assert_eq!(XAssets::total_issuance(&btc_id), 1025);
    })
}

#[test]
fn test_unlock_issue_and_destroy2() {
    ExtBuilder::default().build_no_endowed_and_execute(|| {
        let a: u64 = 1; // accountid
        let btc_id = X_BTC;

        // issue
        XAssets::issue(&btc_id, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 50);
        assert_eq!(XAssets::total_issuance(&btc_id), 50);

        // reserve
        XAssets::move_balance(
            &btc_id,
            &a,
            AssetType::Usable,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();

        assert_eq!(
            XAssets::asset_balance_of(&a, &btc_id, AssetType::ReservedWithdrawal),
            25
        );
        assert_eq!(XAssets::usable_balance(&a, &btc_id), 25);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 50);

        // unreserve
        XAssets::move_balance(
            &btc_id,
            &a,
            AssetType::ReservedWithdrawal,
            &a,
            AssetType::Usable,
            10,
        )
        .unwrap();

        assert_eq!(
            XAssets::asset_balance_of(&a, &btc_id, AssetType::ReservedWithdrawal),
            15
        );
        assert_eq!(XAssets::usable_balance(&a, &btc_id), 35);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 50);
    })
}

#[test]
fn test_error_issue_and_destroy1() {
    ExtBuilder::default().build_no_endowed_and_execute(|| {
        let a: u64 = 1; // accountid
        let btc_id = X_BTC;
        // issue
        XAssets::issue(&btc_id, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 50);
        assert_eq!(XAssets::total_issuance(&btc_id), 50);
        // destroy first
        // destroy
        assert_noop!(
            XAssets::destroy_reserved_withdrawal(&btc_id, &a, 25),
            XAssetsErr::InsufficientBalance,
        );

        assert_noop!(
            XAssets::move_balance(
                &btc_id,
                &a,
                AssetType::Usable,
                &a,
                AssetType::ReservedWithdrawal,
                100
            ),
            AssetErr::NotEnough
        );

        // lock first
        XAssets::move_balance(
            &btc_id,
            &a,
            AssetType::Usable,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();
        // destroy
        assert_ok!(XAssets::destroy_reserved_withdrawal(&btc_id, &a, 25));
    })
}

#[test]
fn test_error_issue_and_destroy2() {
    ExtBuilder::default().build_no_endowed_and_execute(|| {
        let a: u64 = 1; // accountid
        let btc_id = X_BTC;
        // issue
        XAssets::issue(&btc_id, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 50);
        assert_eq!(XAssets::total_issuance(&btc_id), 50);
        // overflow
        let i: i32 = -1;

        assert_noop!(
            XAssets::move_balance(
                &btc_id,
                &a,
                AssetType::Usable,
                &a,
                AssetType::ReservedWithdrawal,
                i as Balance,
            ),
            AssetErr::NotEnough
        );

        assert_noop!(
            XAssets::issue(&btc_id, &a, i as Balance),
            XAssetsErr::Overflow
        );
    })
}

#[test]
fn test_error_issue_and_destroy3() {
    ExtBuilder::default().build_no_endowed_and_execute(|| {
        let a: u64 = 1; // accountid
        let btc_id = X_BTC;
        // lock or destroy without init
        assert_noop!(
            XAssets::destroy_reserved_withdrawal(&btc_id, &a, 25),
            XAssetsErr::InsufficientBalance
        );

        assert_noop!(
            XAssets::move_balance(
                &btc_id,
                &a,
                AssetType::Usable,
                &a,
                AssetType::ReservedWithdrawal,
                25
            ),
            AssetErr::NotEnough
        );

        XAssets::issue(&btc_id, &a, 0).unwrap();
        assert_noop!(
            XAssets::destroy_reserved_withdrawal(&btc_id, &a, 25),
            XAssetsErr::InsufficientBalance
        );

        assert_noop!(
            XAssets::move_balance(
                &btc_id,
                &a,
                AssetType::Usable,
                &a,
                AssetType::ReservedWithdrawal,
                25
            ),
            AssetErr::NotEnough
        );

        XAssets::issue(&btc_id, &a, 100).unwrap();

        XAssets::move_balance(
            &btc_id,
            &a,
            AssetType::Usable,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();

        assert_ok!(XAssets::destroy_reserved_withdrawal(&btc_id, &a, 25));
    })
}

#[test]
fn test_balance_btree_map() {
    ExtBuilder::default().build_and_execute(|| {
        let a: u64 = 100; // accountid
        let b: u64 = 200;
        let btc_id = X_BTC;
        assert_eq!(XAssets::total_issuance(&btc_id), 1000);

        let _ = XAssets::issue(&X_BTC, &a, 100);
        let _ = XAssets::move_balance(
            &X_BTC,
            &a,
            AssetType::Usable,
            &a,
            AssetType::ReservedWithdrawal,
            30,
        );
        assert_eq!(AssetBalance::<Test>::get(&a, &btc_id).len(), 2);
        assert_eq!(TotalAssetBalance::<Test>::get(&btc_id).len(), 2);

        let _ = XAssets::move_balance(
            &X_BTC,
            &a,
            AssetType::ReservedWithdrawal,
            &a,
            AssetType::Usable,
            10,
        );
        let _ = XAssets::move_balance(
            &X_BTC,
            &a,
            AssetType::ReservedWithdrawal,
            &b,
            AssetType::Usable,
            20,
        );
        assert_eq!(AssetBalance::<Test>::get(&a, &btc_id).len(), 1);
        assert_eq!(TotalAssetBalance::<Test>::get(&btc_id).len(), 1);
        assert_eq!(XAssets::usable_balance(&a, &X_BTC,), 80);
        assert_eq!(XAssets::usable_balance(&b, &X_BTC,), 20);
        assert_eq!(XAssets::total_issuance(&X_BTC), 1100); // 1000 + 100
    })
}
/* todo! Fix EventRecord
#[test]
fn test_account_init() {
    ExtBuilder::default().build_and_execute(|| {
        let a: u64 = 999; // accountid
        let id1: u64 = 1000;
        let btc_id = X_BTC;
        assert_eq!(XAssets::total_issuance(&btc_id), 1000);

        // issue init
        let _ = XAssets::issue(&X_BTC, &a, 100);
        assert!(System::events().contains(&EventRecord {
            phase: Phase::Initialization,
            event: Event::System(frame_system::Event::<Test>::NewAccount(a)),
            topics: vec![],
        }));

        // transfer token init
        assert_ok!(XAssets::transfer(Origin::signed(a), id1, btc_id, 25,));
        assert!(System::events().contains(&EventRecord {
            phase: Phase::Initialization,
            event: Event::System(frame_system::Event::<Test>::NewAccount(id1)),
            topics: vec![],
        }));
    })
}

#[test]
fn test_transfer_not_init() {
    ExtBuilder::default().build_and_execute(|| {
        fn check_only_one_new_account(new_id: u64) {
            let count = System::events()
                .iter()
                .filter(|e| {
                    **e == EventRecord {
                        phase: Phase::Initialization,
                        event: Event::System(frame_system::Event::<Test>::NewAccount(new_id)),
                        topics: vec![],
                    }
                })
                .count();
            assert_eq!(count, 1);
        }

        let a: u64 = 1; // accountid
        let new_id: u64 = 1000;
        let btc_id = X_BTC;
        XAssets::issue(&btc_id, &a, 50).unwrap();
        assert_ok!(XAssets::transfer(Origin::signed(a), new_id, btc_id, 25,));
        check_only_one_new_account(new_id);

        assert_ok!(XAssets::transfer(Origin::signed(a), new_id, btc_id, 25,));
        check_only_one_new_account(new_id);

        {
            let _ = <Test as Config>::Currency::deposit_creating(&a, 1000);
            let _ = <Test as Config>::Currency::transfer(Origin::signed(a), new_id, 10);
        }
        check_only_one_new_account(new_id);

        assert_eq!(System::consumers(&new_id), 1);
        assert_ok!(XAssets::transfer(Origin::signed(new_id), a, btc_id, 50,));
        assert_eq!(System::consumers(&new_id), 0);
        assert_ok!(XAssets::transfer(Origin::signed(a), new_id, btc_id, 50,));
        check_only_one_new_account(new_id);
    })
}
*/

#[test]
fn test_transfer_token() {
    ExtBuilder::default().build_no_endowed_and_execute(|| {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_id = X_BTC;
        // issue 50 to account 1
        XAssets::issue(&btc_id, &a, 50).unwrap();
        // transfer
        XAssets::transfer(Origin::signed(a), b, btc_id, 25).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 25);
        assert_eq!(XAssets::usable_balance(&b, &btc_id), 25);

        assert_noop!(
            XAssets::transfer(Origin::signed(a), b, btc_id, 50,),
            XAssetsErr::InsufficientBalance
        );
    })
}

#[test]
fn test_transfer_to_self() {
    ExtBuilder::default().build_no_endowed_and_execute(|| {
        let a: u64 = 1; // accountid
        let btc_id = X_BTC;
        // issue 50 to account 1
        XAssets::issue(&btc_id, &a, 50).unwrap();
        // transfer
        assert_ok!(XAssets::transfer(Origin::signed(a), a, btc_id, 25,));

        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_id), 50);
    })
}

#[test]
fn test_move() {
    ExtBuilder::default().build_and_execute(|| {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_id = X_BTC;
        XAssets::move_usable_balance(&btc_id, &a, &b, 100).unwrap();
        assert_noop!(
            XAssets::move_usable_balance(&btc_id, &a, &b, 1000),
            AssetErr::NotEnough
        );
        assert_eq!(XAssets::usable_balance(&a, &btc_id), 0);
        assert_eq!(XAssets::usable_balance(&b, &btc_id), 200 + 100);

        let token = X_BTC;
        assert_noop!(
            XAssets::move_usable_balance(&token, &a, &b, 100),
            AssetErr::NotEnough
        );

        XAssets::issue(&token, &a, 100).unwrap();
        XAssets::move_usable_balance(&token, &a, &b, 100).unwrap();
        assert_noop!(
            XAssets::move_usable_balance(&token, &a, &b, 1000),
            AssetErr::NotEnough
        );

        assert_eq!(XAssets::usable_balance(&a, &token), 0);
        assert_eq!(XAssets::usable_balance(&b, &token), 200 + 100 + 100);
    })
}
