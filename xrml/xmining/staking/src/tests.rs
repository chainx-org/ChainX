// Copyright 2018 Chainpool.
//! Tests for the module.

#![cfg(test)]

use super::*;
use mock::{new_test_ext, Origin, Session, Staking, System, XAccounts, XAssets};
use primitives::testing::UintAuthorityId;
use runtime_io::with_externalities;
use runtime_support::{assert_noop, assert_ok};

#[test]
fn register_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(Origin::signed(1), b"name".to_vec(),));

        assert_noop!(
            Staking::register(Origin::signed(1), b"name".to_vec(),),
            "Cannot register if transactor is an intention already."
        );

        assert_ok!(Staking::register(Origin::signed(2), b"name".to_vec(),));
    });
}

#[test]
fn refresh_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(Origin::signed(1), b"name".to_vec(),));

        assert_ok!(Staking::refresh(
            Origin::signed(1),
            Some(b"new.name".to_vec()),
            Some(true),
            Some(UintAuthorityId(123).into()),
            None
        ));
        assert_eq!(XAccounts::intention_props_of(&1).is_active, true);
        assert_eq!(XAccounts::intention_props_of(&1).url, b"new.name".to_vec());

        assert_noop!(
            Staking::refresh(
                Origin::signed(2),
                Some(b"new.url".to_vec()),
                Some(false),
                Some(UintAuthorityId(124).into()),
                None
            ),
            "Cannot refresh if transactor is not an intention."
        );
    });
}

#[test]
fn nominate_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(Origin::signed(1), b"name".to_vec(),));

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::nominate(Origin::signed(2), 1.into(), 15, vec![]));

        assert_eq!(XAssets::pcx_free_balance(&2), 20 - 15);
        assert_eq!(
            Staking::nomination_record_of(&2, &1),
            NominationRecord {
                nomination: 15,
                last_vote_weight: 0,
                last_vote_weight_update: 2,
                revocations: vec![],
            }
        );
    });
}

#[test]
fn unnominate_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(Origin::signed(1), b"name".to_vec(),));
        assert_ok!(Staking::nominate(Origin::signed(2), 1.into(), 15, vec![]));

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_noop!(
            Staking::unnominate(Origin::signed(2), 1.into(), 10_000, vec![]),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28801);
        Session::check_rotate_session(System::block_number());
        assert_noop!(
            Staking::unnominate(Origin::signed(2), 1.into(), 10_000, vec![]),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28802);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(Origin::signed(2), 1.into(), 10, vec![]));

        assert_eq!(
            Staking::nomination_record_of(&2, &1),
            NominationRecord {
                nomination: 5,
                last_vote_weight: 432015,
                last_vote_weight_update: 28802,
                revocations: vec![(28803, 10)],
            }
        );

        System::set_block_number(28803);
        Session::check_rotate_session(System::block_number());
        assert_ok!(Staking::unnominate(Origin::signed(2), 1.into(), 5, vec![]));

        assert_eq!(
            Staking::nomination_record_of(&2, &1),
            NominationRecord {
                nomination: 0,
                last_vote_weight: 432020,
                last_vote_weight_update: 28803,
                revocations: vec![(28803, 10), (28804, 5)],
            }
        );
    });
}

#[test]
fn new_trustees_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::register(Origin::signed(1), b"name".to_vec()));
        assert_ok!(Staking::nominate(Origin::signed(1), 1.into(), 5, vec![]));
        assert_ok!(Staking::refresh(
            Origin::signed(1),
            Some(b"new.name".to_vec()),
            Some(true),
            Some(UintAuthorityId(123).into()),
            None
        ));
        assert_ok!(Staking::setup_trustee(
            Origin::signed(1),
            Chain::Bitcoin,
            b"about".to_vec(),
            TrusteeEntity::Bitcoin(vec![0; 33]),
            TrusteeEntity::Bitcoin(vec![0; 33]),
        ));

        System::set_block_number(10);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(11);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(12);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(13);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(14);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(15);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(16);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(17);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAccounts::trustee_intentions(), [10, 20, 30, 40]);

        System::set_block_number(18);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Session::current_index(), 10);
        assert_eq!(XAccounts::trustee_intentions(), [30, 40, 20, 10, 1]);
    });
}

/*
#[test]
fn unfreeze_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(Staking::register(Origin::signed(2), b"name".to_vec(),));

        assert_ok!(Staking::refresh(
            Origin::signed(2),
            Some(b"domainname".to_vec()),
            Some(true),
            Some(UintAuthorityId(123).into()),
            None
        ));
        assert_ok!(Staking::nominate(Origin::signed(2), 2.into(), 20, vec![]));

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

        // assert_ok!(Staking::set_bonding_duration(3.into()));

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
        assert_ok!(Staking::register(Origin::signed(1), b"name".to_vec(),));
        assert_ok!(Staking::refresh(
            Origin::signed(2),
            None,
            Some(true),
            None,
            None
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
*/
