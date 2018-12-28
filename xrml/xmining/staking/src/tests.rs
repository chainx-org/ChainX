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
            b"domainname".to_vec(),
            1,
            vec![],
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
                revocations: vec![],
            }
        );

        assert_eq!(XAccounts::intention_props_of(&2).is_active, false);

        assert_noop!(
            Staking::register(
                Origin::signed(1),
                b"alice".to_vec(),
                2,
                b"name".to_vec(),
                b"domainname".to_vec(),
                1,
                vec![]
            ),
            "Cannot register an intention repeatedly."
        );

        assert_noop!(
            Staking::register(
                Origin::signed(2),
                b"alice".to_vec(),
                2,
                b"name".to_vec(),
                b"domainname".to_vec(),
                1,
                vec![]
            ),
            "Transactor mismatches the owner of given cert name."
        );

        assert_ok!(Staking::register(
            Origin::signed(1),
            b"alice".to_vec(),
            1,
            b"name".to_vec(),
            b"domainname".to_vec(),
            1,
            vec![]
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
            b"domainname".to_vec(),
            1,
            vec![]
        ));

        assert_ok!(Staking::refresh(
            Origin::signed(2),
            b"new.name".to_vec(),
            true
        ));
        assert_eq!(XAccounts::intention_props_of(&2).is_active, true);
        assert_eq!(XAccounts::intention_props_of(&2).url, b"new.name".to_vec());

        assert_ok!(Staking::refresh(
            Origin::signed(2),
            b"new.url".to_vec(),
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
            b"domainname".to_vec(),
            1,
            vec![]
        ));

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::nominate(Origin::signed(2), 2.into(), 15, vec![]));

        assert_eq!(XAssets::pcx_free_balance(&2), 20 - 15);
        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 100_000_000 + 15,
                last_vote_weight: 100_000_000,
                last_vote_weight_update: 2,
                revocations: vec![],
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
            b"domainname".to_vec(),
            1,
            vec![]
        ));

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_noop!(
            Staking::unnominate(Origin::signed(2), 2.into(), 10_000, vec![]),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28801);
        Session::check_rotate_session(System::block_number());
        assert_noop!(
            Staking::unnominate(Origin::signed(2), 2.into(), 10_000, vec![]),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28802);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(
            Origin::signed(2),
            2.into(),
            10_000,
            vec![]
        ));

        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 100_000_000 - 10_000,
                last_vote_weight: 100_000_000 * (28802 - 1),
                last_vote_weight_update: 28802,
                revocations: vec![(28803, 10_000)],
            }
        );

        assert_ok!(Staking::set_bonding_duration(3.into()));

        System::set_block_number(28803);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(
            Origin::signed(2),
            2.into(),
            10_000,
            vec![]
        ));

        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 100_000_000 - 10_000 - 10_000,
                last_vote_weight: 100_000_000 * (28802 - 1) + (100_000_000 - 10_000) * 1,
                last_vote_weight_update: 28803,
                revocations: vec![(28803, 10_000), (28806, 10000)],
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
            b"domainname".to_vec(),
            10,
            vec![]
        ));

        assert_ok!(Staking::refresh(
            Origin::signed(2),
            b"domainname".to_vec(),
            true
        ));

        System::set_block_number(28801);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 1);
        assert_noop!(
            Staking::unnominate(Origin::signed(2), 2.into(), 10_000, vec![]),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28802);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(
            Origin::signed(2),
            2.into(),
            10_000,
            vec![]
        ));
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 1 + 1);
        // No refund
        assert_noop!(
            Staking::unfreeze(Origin::signed(2), 2.into(), 0),
            "The requested revocation is not due yet."
        );

        System::set_block_number(28803);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 1 + 1 + 1);
        assert_ok!(Staking::unfreeze(Origin::signed(2), 2.into(), 0));
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 1 + 1 + 1 + 10_000);

        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 1000_000_000 - 10_000,
                last_vote_weight: 1000_000_000 * (28802 - 1),
                last_vote_weight_update: 28802,
                revocations: vec![],
            }
        );

        assert_ok!(Staking::set_bonding_duration(3.into()));

        System::set_block_number(28804);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(
            Origin::signed(2),
            2.into(),
            10_000,
            vec![]
        ));

        System::set_block_number(28805);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(
            Origin::signed(2),
            2.into(),
            10_000,
            vec![]
        ));
        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 1000_000_000 - 10_000 - 10_000 - 10_000,
                last_vote_weight: 28803999960000,
                last_vote_weight_update: 28805,
                revocations: vec![(28807, 10_000), (28808, 10000)],
            }
        );

        System::set_block_number(28809);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unfreeze(Origin::signed(2), 2.into(), 1));
        assert_eq!(
            Staking::nomination_record_of(&2, &2),
            NominationRecord {
                nomination: 1000_000_000 - 10_000 - 10_000 - 10_000,
                last_vote_weight: 28803999960000,
                last_vote_weight_update: 28805,
                revocations: vec![(28807, 10_000)],
            }
        );
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
            b"domainname".to_vec(),
            10,
            vec![]
        ));
        assert_ok!(Staking::refresh(
            Origin::signed(2),
            b"domainname".to_vec(),
            true
        ));

        assert_eq!(XAccounts::intention_props_of(&2).is_active, true);

        assert_eq!(XAssets::pcx_free_balance(&2), 20);

        System::set_block_number(1);
        assert_eq!(XAssets::pcx_free_balance(&2), 20);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 1);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 1 + 1);
        assert_ok!(Staking::nominate(Origin::signed(2), 2.into(), 10, vec![]));

        assert_eq!(XAssets::pcx_free_balance(&2), 22 - 10);

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 12 + 1);

        assert_ok!(Staking::claim(Origin::signed(2), 2.into()));
        assert_eq!(XAssets::pcx_free_balance(&2), 13 + 9 * 3);
    });
}
