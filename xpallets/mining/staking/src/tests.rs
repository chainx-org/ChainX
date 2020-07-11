use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};

fn t_issue_pcx(to: AccountId, value: Balance) -> DispatchResult {
    XAssets::pcx_issue(&to, value)
}

fn t_register(who: AccountId) -> DispatchResult {
    XStaking::register(Origin::signed(who))
}

fn t_bond(who: AccountId, target: AccountId, value: Balance) -> DispatchResult {
    XStaking::bond(Origin::signed(who), target, value, b"memo".as_ref().into())
}

fn t_rebond(who: AccountId, from: AccountId, to: AccountId, value: Balance) -> DispatchResult {
    XStaking::rebond(
        Origin::signed(who),
        from,
        to,
        value,
        b"memo".as_ref().into(),
    )
}

fn t_unbond(who: AccountId, target: AccountId, value: Balance) -> DispatchResult {
    XStaking::unbond(Origin::signed(who), target, value, b"memo".as_ref().into())
}

fn t_withdraw_unbonded(who: AccountId, unbonded_index: UnbondedIndex) -> DispatchResult {
    XStaking::withdraw_unbonded(Origin::signed(who), unbonded_index)
}

fn t_system_block_number_inc(number: BlockNumber) {
    System::set_block_number((System::block_number() + number).into());
}

fn t_start_session(session_index: SessionIndex) {
    assert_eq!(
        <Period as Get<BlockNumber>>::get(),
        1,
        "start_session can only be used with session length 1."
    );
    for i in Session::current_index()..session_index {
        // XStaking::on_finalize(System::block_number());
        System::set_block_number((i + 1).into());
        Timestamp::set_timestamp(System::block_number() * 1000 + INIT_TIMESTAMP);
        Session::on_initialize(System::block_number());
        // XStaking::on_initialize(System::block_number());
    }

    assert_eq!(Session::current_index(), session_index);
}

#[test]
fn bond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(XAssets::pcx_free_balance(&1), 90);
        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 20,
                last_total_vote_weight: 0,
                last_total_vote_weight_update: 0,
            }
        );
        assert_eq!(System::block_number(), 1);

        t_system_block_number_inc(1);
        assert_ok!(t_bond(1, 2, 10));

        assert_eq!(XAssets::pcx_free_balance(&1), 80);
        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 30,
                last_total_vote_weight: 40,
                last_total_vote_weight_update: 2,
            }
        );
        assert_eq!(
            <Nominations<Test>>::get(1, 2),
            NominatorLedger {
                nomination: 10,
                last_vote_weight: 0,
                last_vote_weight_update: 2,
            }
        );
    });
}

#[test]
fn unbond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_err!(t_unbond(1, 2, 50), Error::<Test>::InvalidUnbondValue);

        t_system_block_number_inc(1);

        assert_ok!(t_bond(1, 2, 10));

        t_system_block_number_inc(1);

        assert_ok!(t_unbond(1, 2, 5));

        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 25,
                last_total_vote_weight: 30 + 20 * 2,
                last_total_vote_weight_update: 3,
            }
        );

        assert_eq!(
            <Nominations<Test>>::get(1, 2),
            NominatorLedger {
                nomination: 5,
                last_vote_weight: 10,
                last_vote_weight_update: 3,
            }
        );

        assert_eq!(
            <Nominators<Test>>::get(1),
            NominatorProfile {
                last_rebond: None,
                unbonded_chunks: vec![Unbonded {
                    value: 5,
                    locked_until: 50 * 12 * 24 * 3 + 3
                }],
            }
        );
    });
}

#[test]
fn rebond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_err!(
            XStaking::unbond(Origin::signed(1), 2, 50, b"memo".as_ref().into()),
            Error::<Test>::InvalidUnbondValue
        );

        // Block 2
        t_system_block_number_inc(1);

        assert_ok!(t_bond(1, 2, 10));

        // Block 3
        t_system_block_number_inc(1);

        assert_ok!(t_rebond(1, 2, 3, 5));

        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 25,
                last_total_vote_weight: 30 + 40,
                last_total_vote_weight_update: 3,
            }
        );

        assert_eq!(
            <ValidatorLedgers<Test>>::get(3),
            ValidatorLedger {
                total: 30 + 5,
                last_total_vote_weight: 30 * 3,
                last_total_vote_weight_update: 3,
            }
        );

        assert_eq!(
            <Nominations<Test>>::get(1, 2),
            NominatorLedger {
                nomination: 5,
                last_vote_weight: 10,
                last_vote_weight_update: 3,
            }
        );

        assert_eq!(
            <Nominations<Test>>::get(1, 3),
            NominatorLedger {
                nomination: 5,
                last_vote_weight: 0,
                last_vote_weight_update: 3,
            }
        );

        assert_eq!(
            <Nominators<Test>>::get(1),
            NominatorProfile {
                last_rebond: Some(3),
                unbonded_chunks: vec![]
            }
        );
    });
}

#[test]
fn withdraw_unbond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        t_system_block_number_inc(1);

        assert_ok!(t_bond(1, 2, 10));
        assert_eq!(XAssets::pcx_free_balance(&1), 80);

        t_system_block_number_inc(1);

        assert_ok!(t_unbond(1, 2, 5));
        assert_eq!(XAssets::pcx_free_balance(&1), 80);

        assert_eq!(
            <Nominators<Test>>::get(1),
            NominatorProfile {
                last_rebond: None,
                unbonded_chunks: vec![Unbonded {
                    value: 5,
                    locked_until: DEFAULT_BONDING_DURATION + 3
                }]
            }
        );

        t_system_block_number_inc(DEFAULT_BONDING_DURATION);
        assert_err!(
            t_withdraw_unbonded(1, 0),
            Error::<Test>::UnbondRequestNotYetDue
        );

        t_system_block_number_inc(1);

        assert_ok!(t_withdraw_unbonded(1, 0),);
        assert_eq!(XAssets::pcx_free_balance(&1), 85);
    });
}

fn t_make_a_validator_candidate(who: AccountId, self_bonded: Balance) {
    assert_ok!(t_issue_pcx(who, self_bonded));
    assert_ok!(t_register(who));
    assert_ok!(t_bond(who, who, self_bonded));
}

#[test]
fn regular_staking_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        // Block 1
        t_start_session(1);
        assert_eq!(XStaking::current_era(), Some(0));

        t_make_a_validator_candidate(5, 500);
        t_make_a_validator_candidate(6, 600);
        t_make_a_validator_candidate(7, 700);
        t_make_a_validator_candidate(8, 800);

        t_start_session(2);
        assert_eq!(XStaking::current_era(), Some(1));
        assert_eq!(Session::validators(), vec![4, 3, 2, 1]);

        // TODO: figure out the exact session for validators change.
        // sessions_per_era = 3
        //
        // The new session validators will take effect until new_era's start_session_index + 1.
        //
        // [new_era]current_era:1, start_session_index:3, maybe_new_validators:Some([4, 3, 2, 1, 8, 7])
        // Session Validators: [4, 3, 2, 1]
        //
        // [start_session]:start_session:3, next_active_era:1
        // [new_session]session_index:4, current_era:Some(1)
        // Session Validators: [8, 7, 6, 5, 4, 3]  <--- Session index is still 3
        t_start_session(3);
        assert_eq!(XStaking::current_era(), Some(1));
        assert_eq!(Session::current_index(), 3);
        assert_eq!(Session::validators(), vec![8, 7, 6, 5, 4, 3]);

        t_start_session(4);
        assert_eq!(XStaking::current_era(), Some(1));
        assert_ok!(XStaking::chill(Origin::signed(6)));
        assert_eq!(Session::validators(), vec![8, 7, 6, 5, 4, 3]);

        t_start_session(5);
        assert_eq!(XStaking::current_era(), Some(2));
        assert_ok!(XStaking::chill(Origin::signed(5)));
        assert_eq!(Session::validators(), vec![8, 7, 6, 5, 4, 3]);

        t_start_session(6);
        assert_eq!(XStaking::current_era(), Some(2));
        assert_eq!(XStaking::is_chilled(&5), true);
        assert_eq!(XStaking::is_chilled(&6), true);
        assert_eq!(Session::validators(), vec![8, 7, 5, 4, 3, 2]);

        t_start_session(7);
        assert_eq!(XStaking::current_era(), Some(2));
        assert_eq!(Session::validators(), vec![8, 7, 5, 4, 3, 2]);

        t_start_session(8);
        assert_eq!(XStaking::current_era(), Some(3));
        assert_eq!(Session::validators(), vec![8, 7, 5, 4, 3, 2]);

        t_start_session(9);
        assert_eq!(XStaking::current_era(), Some(3));
        assert_eq!(Session::validators(), vec![8, 7, 4, 3, 2, 1]);
    })
}

// TODO:
// claim_test
// slash_test
// force_new_era_test
