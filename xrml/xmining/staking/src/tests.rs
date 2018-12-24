// Copyright 2018 Chainpool.
//! Tests for the module.

#![cfg(test)]

use super::*;
use mock::{new_test_ext, Origin, Session, Staking, System, XAccounts, XAssets};
use runtime_io::with_externalities;

#[test]
fn register_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAccounts::issue(b"alice".to_vec(), 1, 1));

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            2,
            b"name".to_vec(),
            b"url".to_vec(),
            1,
        ));

        assert_eq!(XAccounts::remaining_shares_of(b"alice".to_vec()), 49);
        assert_eq!(XAssets::pcx_free_balance(&2), 20);
        assert_eq!(XAssets::pcx_total_balance(&2), 100_000_000 + 20);
        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 100_000_000,
                last_vote_weight: 0,
                last_vote_weight_update: 1,
            }
        );

        assert_eq!(XAccounts::intention_props_of(&2).is_active, false);

        assert_noop!(
            Staking::register(
                Origin::signed(1),
                b"alice".to_vec(),
                2,
                b"name".to_vec(),
                b"url".to_vec(),
                1,
            ),
            "Cannot register an intention repeatedly."
        );

        assert_noop!(
            Staking::register(
                Origin::signed(2),
                b"alice".to_vec(),
                2,
                b"name".to_vec(),
                b"url".to_vec(),
                1,
            ),
            "Transactor mismatches the owner of given cert name."
        );

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            1,
            b"name".to_vec(),
            b"url".to_vec(),
            1,
        ));
    });
}

#[test]
fn refresh_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAccounts::issue(b"alice".to_vec(), 1, 1));

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            2,
            b"name".to_vec(),
            b"url".to_vec(),
            1,
        ));

        assert_ok!(Staking::refresh(
            Origin::signed(2),
            b"new_url".to_vec(),
            true
        ));
        assert_eq!(XAccounts::intention_props_of(&2).is_active, true);
        assert_eq!(XAccounts::intention_props_of(&2).url, b"new_url".to_vec());

        assert_ok!(Staking::refresh(
            Origin::signed(2),
            b"new_url".to_vec(),
            false
        ));
        assert_eq!(XAccounts::intention_props_of(&2).is_active, false);
    });
}

#[test]
fn nominate_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAccounts::issue(b"alice".to_vec(), 1, 1));

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            2,
            b"name".to_vec(),
            b"url".to_vec(),
            1,
        ));

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::nominate(Origin::signed(2), 2.into(), 15));

        assert_eq!(XAssets::pcx_free_balance(&2), 20 - 15);
        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 100_000_000 + 15,
                last_vote_weight: 100_000_000,
                last_vote_weight_update: 2,
            }
        );
    });
}

#[test]
fn unnominate_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAccounts::issue(b"alice".to_vec(), 1, 1));

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            2,
            b"name".to_vec(),
            b"url".to_vec(),
            1,
        ));

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_noop!(
            Staking::unnominate(Origin::signed(2), 2.into(), 10_000),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28801);
        Session::check_rotate_session(System::block_number());
        assert_noop!(
            Staking::unnominate(Origin::signed(2), 2.into(), 10_000),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28802);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(Origin::signed(2), 2.into(), 10_000));
        assert_eq!(Staking::remaining_frozen_of(&2), [28803]);

        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 100_000_000 - 10_000,
                last_vote_weight: 100_000_000 * (28802 - 1),
                last_vote_weight_update: 28802,
            }
        );
    });
}

#[test]
fn unfreeze_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAccounts::issue(b"alice".to_vec(), 1, 1));

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            2,
            b"name".to_vec(),
            b"url".to_vec(),
            1,
        ));

        assert_ok!(Staking::refresh(Origin::signed(2), b"url".to_vec(), true));

        System::set_block_number(28802);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(Origin::signed(2), 2.into(), 10_000));
        assert_eq!(XAssets::pcx_free_balance(&2), 30);
        assert_ok!(Staking::unfreeze(Origin::signed(2)));
        // No refund
        assert_eq!(XAssets::pcx_free_balance(&2), 30);

        System::set_block_number(28803);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 30 + 9);
        // No refund
        assert_ok!(Staking::unfreeze(Origin::signed(2)));

        System::set_block_number(28804);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 30 + 9 + 9);
        assert_ok!(Staking::unfreeze(Origin::signed(2)));
        assert_eq!(XAssets::pcx_free_balance(&2), 30 + 9 + 9 + 10_000);
    });
}

#[test]
fn claim_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XAccounts::issue(b"alice".to_vec(), 1, 1));

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            2,
            b"name".to_vec(),
            b"url".to_vec(),
            1,
        ));
        assert_ok!(Staking::refresh(Origin::signed(2), b"url".to_vec(), true));

        assert_eq!(XAccounts::intention_props_of(&2).is_active, true);

        assert_eq!(XAssets::pcx_free_balance(&2), 20);

        System::set_block_number(1);
        assert_eq!(XAssets::pcx_free_balance(&2), 20);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 10);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 30 + 10);
        assert_ok!(Staking::nominate(Origin::signed(2), 2.into(), 10));

        assert_eq!(XAssets::pcx_free_balance(&2), 40 - 10);

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 30 + 10);

        assert_ok!(Staking::claim(Origin::signed(2), 2.into()));
        assert_eq!(XAssets::pcx_free_balance(&2), 40 + 90 * 3);
    });
}
