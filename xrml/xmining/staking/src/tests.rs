// Copyright 2018-2019 Chainpool.
//! Tests for the module.

#![cfg(test)]

use super::mock::*;
use super::*;

use primitives::testing::UintAuthorityId;
use runtime_io::with_externalities;
use support::{assert_noop, assert_ok};

#[test]
fn register_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::register(Origin::signed(1), b"name".to_vec(),));

        assert_noop!(
            XStaking::register(Origin::signed(1), b"name".to_vec(),),
            "Cannot register if transactor is an intention already."
        );
    });
}

#[test]
fn register_an_existing_name_should_not_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::register(Origin::signed(1), b"name".to_vec(),));
        assert_noop!(
            XStaking::register(Origin::signed(2), b"name".to_vec()),
            "This name has already been taken."
        );
    });
}

#[test]
fn refresh_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::register(Origin::signed(1), b"name".to_vec(),));

        assert_ok!(XStaking::refresh(
            Origin::signed(1),
            Some(b"new.name".to_vec()),
            Some(true),
            Some(UintAuthorityId(123).into()),
            None
        ));
        assert_eq!(XAccounts::intention_props_of(&1).is_active, true);
        assert_eq!(XAccounts::intention_props_of(&1).url, b"new.name".to_vec());

        assert_noop!(
            XStaking::refresh(
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
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::register(Origin::signed(1), b"name".to_vec(),));

        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 10, vec![]));
        assert_ok!(XStaking::nominate(Origin::signed(2), 1.into(), 15, vec![]));

        assert_eq!(XAssets::pcx_free_balance(&2), 20 - 15);
        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 1)).unwrap(),
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
fn renominate_by_intention_should_not_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::register(Origin::signed(1), b"name".to_vec(),));
        assert_ok!(XStaking::register(Origin::signed(3), b"name3".to_vec(),));

        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 5, vec![]));

        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());
        assert_noop!(
            XStaking::renominate(Origin::signed(1), 1.into(), 3.into(), 3, b"memo".to_vec()),
            "Cannot renominate the intention self-bonded."
        );
    });
}

#[test]
fn renominate_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::set_bonding_duration(0));

        assert_ok!(XStaking::register(Origin::signed(1), b"name".to_vec(),));
        assert_ok!(XStaking::register(Origin::signed(3), b"name3".to_vec(),));

        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 5, vec![]));
        assert_ok!(XStaking::nominate(Origin::signed(3), 3.into(), 5, vec![]));

        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::nominate(Origin::signed(2), 1.into(), 15, vec![]));

        assert_eq!(XAssets::pcx_free_balance(&2), 20 - 15);
        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 1)).unwrap(),
            NominationRecord {
                nomination: 15,
                last_vote_weight: 0,
                last_vote_weight_update: 2,
                revocations: vec![],
            }
        );

        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::renominate(
            Origin::signed(2),
            1.into(),
            3.into(),
            10,
            b"memo".to_vec()
        ));
        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 1)).unwrap(),
            NominationRecord {
                nomination: 5,
                last_vote_weight: 15,
                last_vote_weight_update: 3,
                revocations: vec![],
            }
        );
        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 3)).unwrap(),
            NominationRecord {
                nomination: 10,
                last_vote_weight: 0,
                last_vote_weight_update: 3,
                revocations: vec![],
            }
        );

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::renominate(
            Origin::signed(2),
            1.into(),
            3.into(),
            5,
            b"memo".to_vec()
        ));
        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 1)).unwrap(),
            NominationRecord {
                nomination: 0,
                last_vote_weight: 20,
                last_vote_weight_update: 4,
                revocations: vec![],
            }
        );
        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 3)).unwrap(),
            NominationRecord {
                nomination: 15,
                last_vote_weight: 10,
                last_vote_weight_update: 4,
                revocations: vec![],
            }
        );
    });
}

#[test]
fn unnominate_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::register(Origin::signed(1), b"name".to_vec(),));
        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 10, vec![]));
        assert_ok!(XStaking::nominate(Origin::signed(2), 1.into(), 15, vec![]));

        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());
        assert_noop!(
            XStaking::unnominate(Origin::signed(2), 1.into(), 10_000, vec![]),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28801);
        XSession::check_rotate_session(System::block_number());
        assert_noop!(
            XStaking::unnominate(Origin::signed(2), 1.into(), 10_000, vec![]),
            "Cannot unnominate if greater than your revokable nomination."
        );

        System::set_block_number(28802);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::unnominate(
            Origin::signed(2),
            1.into(),
            10,
            vec![]
        ));

        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 1)).unwrap(),
            NominationRecord {
                nomination: 5,
                last_vote_weight: 432015,
                last_vote_weight_update: 28802,
                revocations: vec![(28803, 10)],
            }
        );

        System::set_block_number(28803);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::unnominate(Origin::signed(2), 1.into(), 5, vec![]));

        assert_eq!(
            <NominationRecords<Test>>::get(&(2, 1)).unwrap(),
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
        assert_ok!(XStaking::register(Origin::signed(2), b"name".to_vec(),));
        assert_ok!(XStaking::refresh(
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
        XSession::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 20);
        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XAssets::pcx_issue(&2, 10 * 100_000_000));
        assert_eq!(XAssets::pcx_free_balance(&2), 20 + 10 * 100_000_000);
        assert_ok!(XStaking::nominate(
            Origin::signed(2),
            2.into(),
            10 * 100_000_000,
            vec![]
        ));
        assert_eq!(XAssets::pcx_free_balance(&2), 20);
        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&2), 36363656);
        assert_ok!(XStaking::claim(Origin::signed(2), 2.into()));
        assert_eq!(XAssets::pcx_free_balance(&2), 363636383);
    });
}

#[test]
fn multiply_by_rational_should_work() {
    assert_eq!(XStaking::multiply_by_rational(100u64, 1, 3), 33);
    assert_eq!(XStaking::multiply_by_rational(100u64, 2, 3), 66);
    assert_eq!(XStaking::multiply_by_rational(200u64, 1, 3), 66);
    assert_eq!(XStaking::multiply_by_rational(200u64, 1, 5), 40);
    assert_eq!(
        XStaking::multiply_by_rational(u64::max_value(), 2, 5),
        (u128::from(u64::max_value()) * u128::from(2u32) / u128::from(5u32)) as u64
    );
}

#[test]
fn minimum_candidate_threshold_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XStaking::set_minimum_candidate_threshold((
            0,
            10 * 100_000_000
        )));
        assert_ok!(XStaking::register(Origin::signed(6), b"name".to_vec(),));
        assert_ok!(XStaking::refresh(
            Origin::signed(6),
            None,
            Some(true),
            None,
            None
        ));

        assert_ok!(XAssets::pcx_issue(&1, 5));

        assert_ok!(XStaking::nominate(Origin::signed(6), 6.into(), 5, vec![]));
        assert_ok!(XStaking::nominate(Origin::signed(1), 6.into(), 5, vec![]));

        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_eq!(XStaking::is_active(&6), false);

        assert_eq!(
            XStaking::validators(),
            vec![
                (40, 4000000000),
                (30, 3000000000),
                (20, 2000000000),
                (10, 1000000000)
            ]
        );

        assert_ok!(XAssets::pcx_issue(&1, 10 * 100_000_000));
        assert_ok!(XAssets::pcx_issue(&6, 1_000_000_000));
        assert_ok!(XStaking::nominate(
            Origin::signed(6),
            6.into(),
            1_000_000_000,
            vec![]
        ));
        assert_ok!(XStaking::nominate(
            Origin::signed(1),
            6.into(),
            10 * 100_000_000,
            vec![]
        ));

        assert_ok!(XStaking::refresh(
            Origin::signed(6),
            None,
            Some(true),
            None,
            None
        ));

        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());

        assert_eq!(XStaking::is_active(&6), true);

        // Account 6 is a validator
        assert_eq!(
            XStaking::validators(),
            vec![
                (40, 4000000000),
                (30, 3000000000),
                (6, 2000000010),
                (20, 2000000000),
                (10, 1000000000)
            ]
        );
    });
}

#[test]
fn renominate_limitation_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XStaking::set_bonding_duration(2));

        assert_ok!(XStaking::register(Origin::signed(1), b"name1".to_vec(),));
        assert_ok!(XStaking::register(Origin::signed(2), b"name2".to_vec(),));
        assert_ok!(XStaking::register(Origin::signed(3), b"name3".to_vec(),));

        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 5, vec![]));
        assert_ok!(XStaking::nominate(Origin::signed(2), 2.into(), 5, vec![]));
        assert_ok!(XStaking::nominate(Origin::signed(3), 3.into(), 5, vec![]));

        assert_ok!(XStaking::nominate(Origin::signed(4), 1.into(), 10, vec![]));

        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::renominate(
            Origin::signed(4),
            1.into(),
            2.into(),
            3,
            b"memo".to_vec()
        ));

        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());
        assert_noop!(
            XStaking::renominate(Origin::signed(4), 1.into(), 3.into(), 3, b"memo".to_vec()),
            "Cannot renominate if your last renomination is not expired."
        );

        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());
        assert_noop!(
            XStaking::renominate(Origin::signed(4), 1.into(), 3.into(), 3, b"memo".to_vec()),
            "Cannot renominate if your last renomination is not expired."
        );

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::renominate(
            Origin::signed(4),
            1.into(),
            3.into(),
            3,
            b"memo".to_vec()
        ));
    });
}

#[test]
fn upper_bound_of_total_nomination_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XStaking::register(Origin::signed(1), b"name1".to_vec(),));
        assert_ok!(XStaking::register(Origin::signed(2), b"name2".to_vec(),));

        assert_noop!(
            XStaking::nominate(Origin::signed(3), 1.into(), 5, vec![]),
            "Cannot (re)nominate if the target is reaching the upper bound of total nomination."
        );

        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 1, vec![]));

        assert_noop!(
            XStaking::nominate(Origin::signed(3), 1.into(), 10, vec![]),
            "Cannot (re)nominate if the target is reaching the upper bound of total nomination."
        );

        assert_ok!(XStaking::nominate(Origin::signed(3), 1.into(), 9, vec![]));

        assert_ok!(XStaking::nominate(Origin::signed(2), 2.into(), 2, vec![]));
        assert_ok!(XStaking::nominate(Origin::signed(3), 2.into(), 15, vec![]));

        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 1, vec![]));
        assert_noop!(
            XStaking::renominate(Origin::signed(3), 2.into(), 1.into(), 10, vec![]),
            "Cannot (re)nominate if the target is reaching the upper bound of total nomination."
        );
        assert_ok!(XStaking::renominate(
            Origin::signed(3),
            2.into(),
            1.into(),
            9,
            vec![]
        ));
    });
}

#[test]
fn max_unbond_entries_limit_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XStaking::register(Origin::signed(1), b"name1".to_vec(),));
        assert_ok!(XStaking::register(Origin::signed(2), b"name2".to_vec(),));

        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 10, vec![]));

        assert_ok!(XStaking::nominate(Origin::signed(3), 1.into(), 20, vec![]));

        for i in 2..12 {
            System::set_block_number(i);
            XSession::check_rotate_session(System::block_number());
            assert_ok!(XStaking::unnominate(Origin::signed(3), 1.into(), 1, vec![]));
        }

        System::set_block_number(12);
        XSession::check_rotate_session(System::block_number());
        assert_noop!(
            XStaking::unnominate(Origin::signed(3), 1.into(), 1, vec![]),
            "Cannot unnomiate if the limit of max unbond entries is reached."
        );

        assert_ok!(XStaking::unfreeze(Origin::signed(3), 1.into(), 1));

        assert_ok!(XStaking::unnominate(Origin::signed(3), 1.into(), 1, vec![]));
    });
}

#[test]
fn switch_to_u128_when_overflow() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(XStaking::register(Origin::signed(1), b"name1".to_vec(),));
        assert_ok!(XStaking::register(Origin::signed(2), b"name2".to_vec(),));

        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 2, vec![]));
        assert_eq!(
            <NominationRecords<Test>>::get(&(1, 1)).unwrap(),
            NominationRecord {
                nomination: 2,
                last_vote_weight: 0,
                last_vote_weight_update: 1,
                revocations: vec![]
            }
        );

        // user overflow
        //
        // 18446744073709551615 = u64::max_value()
        //
        // 18446744073709551614 = 2 * (u64::max_value() / 2)
        System::set_block_number(1 + u64::max_value() / 2);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::nominate(Origin::signed(3), 1.into(), 1, vec![]));

        assert_eq!(
            <NominationRecords<Test>>::get(&(3, 1)).unwrap(),
            NominationRecord {
                nomination: 1,
                last_vote_weight: 0,
                last_vote_weight_update: 1 + u64::max_value() / 2,
                revocations: vec![]
            }
        );

        System::set_block_number(2 + u64::max_value() / 2);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::claim(Origin::signed(3), 1.into()));

        assert_eq!(
            XStaking::nomination_records(&(3, 1)).unwrap(),
            NominationRecord {
                nomination: 1,
                last_vote_weight: 0,
                last_vote_weight_update: 2 + u64::max_value() / 2,
                revocations: vec![]
            }
        );

        assert_eq!(<Intentions<Test>>::exists(&1), false);
        assert_eq!(
            XStaking::intentions_v1(&1),
            IntentionProfsV1 {
                total_nomination: 3,
                last_total_vote_weight: 2u128 * (2u128 + u128::from(u64::max_value() / 2) - 1u128), // 18446744073709551616
                last_total_vote_weight_update: 2 + u64::max_value() / 2,
            }
        );

        assert_ok!(XStaking::nominate(Origin::signed(1), 1.into(), 1, vec![]));

        assert_eq!(<NominationRecords<Test>>::get(&(1, 1)).is_none(), true);
        assert_eq!(
            XStaking::nomination_records_v1(&(1, 1)).unwrap(),
            NominationRecordV1 {
                nomination: 3,
                last_vote_weight: 2u128 * (2u128 + u128::from(u64::max_value() / 2) - 1u128), // 18446744073709551616
                last_vote_weight_update: 2 + u64::max_value() / 2,
                revocations: vec![]
            }
        );

        assert_eq!(
            XStaking::intentions_v1(&1),
            IntentionProfsV1 {
                total_nomination: 4,
                last_total_vote_weight: 2u128 * (2u128 + u128::from(u64::max_value() / 2) - 1u128), // 18446744073709551616
                last_total_vote_weight_update: 2 + u64::max_value() / 2,
            }
        );

        System::set_block_number(3 + u64::max_value() / 2);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XAssets::pcx_issue(&1, 1000));
        assert_ok!(XStaking::nominate(Origin::signed(3), 1.into(), 1, vec![]));
        assert_eq!(
            XStaking::nomination_records(&(3, 1)).unwrap(),
            NominationRecord {
                nomination: 2,
                last_vote_weight: 1u64,
                last_vote_weight_update: 3 + u64::max_value() / 2,
                revocations: vec![]
            }
        );

        assert_ok!(XStaking::nominate(Origin::signed(2), 2.into(), 2, vec![]));
        assert_ok!(XStaking::renominate(
            Origin::signed(3),
            1.into(),
            2.into(),
            1,
            b"memo".to_vec()
        ));

        assert_eq!(
            XStaking::nomination_records(&(3, 1)).unwrap(),
            NominationRecord {
                nomination: 1,
                last_vote_weight: 1u64,
                last_vote_weight_update: 3 + u64::max_value() / 2,
                revocations: vec![]
            }
        );

        assert_eq!(
            XStaking::nomination_records(&(3, 2)).unwrap(),
            NominationRecord {
                nomination: 1,
                last_vote_weight: 0u64,
                last_vote_weight_update: 3 + u64::max_value() / 2,
                revocations: vec![]
            }
        );

        assert_ok!(XAssets::pcx_issue(&1, 10_000));
        assert_ok!(XStaking::nominate(
            Origin::signed(1),
            1.into(),
            10_000,
            vec![]
        ));
        assert_ok!(XAssets::pcx_issue(&3, 10_000));
        assert_ok!(XStaking::nominate(
            Origin::signed(3),
            1.into(),
            10_000,
            vec![]
        ));

        System::set_block_number(3 + u64::max_value() / 2 + u64::max_value() / 10);
        XSession::check_rotate_session(System::block_number());

        assert_ok!(XStaking::unnominate(Origin::signed(3), 1.into(), 1, vec![]));

        assert_eq!(XStaking::nomination_records(&(3, 1)), None);
        assert_eq!(
            XStaking::nomination_records_v1(&(3, 1)).unwrap(),
            NominationRecordV1 {
                nomination: 10_000,
                last_vote_weight: 1u128 + 10_001u128 * u128::from(u64::max_value() / 10),
                last_vote_weight_update: 3 + u64::max_value() / 2 + u64::max_value() / 10,
                revocations: vec![(3u64 + u64::max_value() / 2 + u64::max_value() / 10 + 1, 1)]
            }
        );

        System::set_block_number(4 + u64::max_value() / 2 + u64::max_value() / 10);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::unnominate(Origin::signed(3), 1.into(), 2, vec![]));
        assert_eq!(
            XStaking::nomination_records_v1(&(3, 1)).unwrap(),
            NominationRecordV1 {
                nomination: 10_000 - 2,
                last_vote_weight: 1u128 + 10_001u128 * u128::from(u64::max_value() / 10) + 10_000,
                last_vote_weight_update: 4 + u64::max_value() / 2 + u64::max_value() / 10,
                revocations: vec![
                    (3u64 + u64::max_value() / 2 + u64::max_value() / 10 + 1, 1),
                    (4u64 + u64::max_value() / 2 + u64::max_value() / 10 + 1, 2)
                ]
            }
        );

        System::set_block_number(4 + u64::max_value() / 2 + u64::max_value() / 10);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XStaking::unfreeze(Origin::signed(3), 1.into(), 0));
        assert_eq!(
            XStaking::nomination_records_v1(&(3, 1)).unwrap(),
            NominationRecordV1 {
                nomination: 10_000 - 2,
                last_vote_weight: 1u128 + 10_001u128 * u128::from(u64::max_value() / 10) + 10_000,
                last_vote_weight_update: 4 + u64::max_value() / 2 + u64::max_value() / 10,
                revocations: vec![(4u64 + u64::max_value() / 2 + u64::max_value() / 10 + 1, 2)]
            }
        );
    });
}
