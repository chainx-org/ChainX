use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok};

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
