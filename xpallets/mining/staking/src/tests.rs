use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok, traits::OnInitialize};

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
                last_total_vote_weight_update: 1,
            }
        );
        t_system_block_number_inc(1);
        assert_ok!(t_bond(1, 2, 10));

        assert_eq!(XAssets::pcx_free_balance(&1), 80);
        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 30,
                last_total_vote_weight: 20,
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
                last_total_vote_weight: 30 + 20,
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

        t_system_block_number_inc(1);

        assert_ok!(t_bond(1, 2, 10));

        t_system_block_number_inc(1);

        assert_ok!(t_rebond(1, 2, 3, 5));

        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 25,
                last_total_vote_weight: 30 + 20,
                last_total_vote_weight_update: 3,
            }
        );

        assert_eq!(
            <ValidatorLedgers<Test>>::get(3),
            ValidatorLedger {
                total: 30 + 5,
                last_total_vote_weight: 30 * 2,
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

fn t_staking_validator_set() -> Vec<AccountId> {
    XStaking::validator_set().collect()
}

#[test]
fn reward_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        t_start_session(1);

        println!("Staking Validators: {:?}", t_staking_validator_set());
        println!("Session Validators: {:?}", Session::validators());

        assert_ok!(t_issue_pcx(5, 500));
        assert_ok!(t_issue_pcx(6, 600));
        assert_ok!(t_issue_pcx(7, 700));
        assert_ok!(t_issue_pcx(8, 800));

        assert_ok!(t_register(5));
        assert_ok!(t_register(6));
        assert_ok!(t_register(7));
        assert_ok!(t_register(8));

        assert_ok!(t_bond(5, 5, 50));
        assert_ok!(t_bond(6, 6, 60));
        assert_ok!(t_bond(7, 7, 70));
        assert_ok!(t_bond(8, 8, 80));

        t_start_session(2);

        println!("Staking Validators: {:?}", t_staking_validator_set());
        println!("Session Validators: {:?}", Session::validators());

        t_start_session(3);

        println!("Staking Validators: {:?}", t_staking_validator_set());
        println!("Session Validators: {:?}", Session::validators());

        t_start_session(4);
        println!("Staking Validators: {:?}", t_staking_validator_set());
        println!("Session Validators: {:?}", Session::validators());
    })
}
