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
fn claim_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(Staking::register(Origin::signed(2), b"name".to_vec(),));
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
        assert_eq!(XAssets::pcx_free_balance(&2), 20);
        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 20);
        assert_ok!(Staking::nominate(Origin::signed(2), 2.into(), 10, vec![]));
        assert_eq!(XAssets::pcx_free_balance(&2), 10);
        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 400000010);
        assert_ok!(Staking::claim(Origin::signed(2), 2.into()));
        assert_eq!(XAssets::pcx_free_balance(&2), 4000000010);
    });
}

#[test]
fn offline_should_slash_and_kick() {
    // Test that an offline validator gets slashed and kicked
    with_externalities(&mut new_test_ext(), || {
        assert_eq!(XAssets::pcx_free_balance(&6), 30);
        assert_ok!(Staking::register(Origin::signed(6), b"name".to_vec(),));
        assert_ok!(Staking::refresh(
            Origin::signed(6),
            None,
            Some(true),
            None,
            None
        ));

        assert_ok!(Staking::register(Origin::signed(10), b"name1".to_vec(),));
        assert_ok!(Staking::refresh(
            Origin::signed(10),
            None,
            Some(true),
            None,
            None
        ));

        assert_ok!(Staking::register(Origin::signed(20), b"name2".to_vec(),));
        assert_ok!(Staking::refresh(
            Origin::signed(20),
            None,
            Some(true),
            None,
            None
        ));

        assert_ok!(Staking::register(Origin::signed(30), b"name3".to_vec(),));
        assert_ok!(Staking::refresh(
            Origin::signed(30),
            None,
            Some(true),
            None,
            None
        ));

        assert_ok!(Staking::register(Origin::signed(40), b"name4".to_vec(),));
        assert_ok!(Staking::refresh(
            Origin::signed(40),
            None,
            Some(true),
            None,
            None
        ));

        assert_ok!(Staking::nominate(Origin::signed(1), 20.into(), 5, vec![]));
        assert_ok!(Staking::nominate(Origin::signed(2), 30.into(), 15, vec![]));
        assert_ok!(Staking::nominate(Origin::signed(3), 40.into(), 15, vec![]));
        assert_ok!(Staking::nominate(Origin::signed(4), 10.into(), 15, vec![]));

        assert_eq!(XAccounts::intention_props_of(&6).is_active, true);
        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::nominate(Origin::signed(4), 6.into(), 5, vec![]));
        let jackpot_addr = Staking::jackpot_accountid_for(&6);
        assert_eq!(XAssets::pcx_free_balance(&jackpot_addr), 0);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());

        // Account 6 is a validator
        assert_eq!(
            Staking::validators(),
            vec![(10, 15), (30, 15), (40, 15), (6, 5), (20, 5)]
        );
        let total_active_stake = 15 + 15 + 15 + 5 + 5;
        let reward = 50_00000000 * 8 / 10 * 5 / total_active_stake;
        let jackpot1 = reward - reward / 10;
        assert_eq!(XAssets::pcx_free_balance(&jackpot_addr), jackpot1);

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        // Validator 6 get slashed immediately
        Staking::on_offline_validator(&6);
        assert_eq!(
            Staking::validators(),
            vec![(10, 15), (30, 15), (40, 15), (6, 5), (20, 5)]
        );

        let reward = 50_00000000 * 8 / 10 * 5 / total_active_stake;
        let jackpot2 = reward - reward / 10;
        assert_eq!(
            XAssets::pcx_free_balance(&jackpot_addr),
            jackpot2 + jackpot1
        );

        System::set_block_number(4);
        Session::check_rotate_session(System::block_number());

        // Validator 6 be kicked
        assert_eq!(
            Staking::validators(),
            [(10, 15), (30, 15), (40, 15), (20, 5)]
        );
        assert_eq!(XAssets::pcx_free_balance(&jackpot_addr), 0);
        assert_eq!(XAccounts::intention_props_of(&2).is_active, false);
    });
}
