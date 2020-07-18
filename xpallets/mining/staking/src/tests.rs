use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use sp_runtime::DispatchResult;

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

        let before_bond = XAssets::pcx_free_balance(&1);
        assert_ok!(t_bond(1, 2, 10));

        assert_eq!(XAssets::pcx_free_balance(&1), before_bond - 10);
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

        let before_bond = XAssets::pcx_free_balance(&1);
        assert_ok!(t_bond(1, 2, 10));
        assert_eq!(XAssets::pcx_free_balance(&1), before_bond - 10);

        t_system_block_number_inc(1);

        assert_ok!(t_unbond(1, 2, 5));
        let before_unbond = XAssets::pcx_free_balance(&1);
        assert_eq!(XAssets::pcx_free_balance(&1), before_unbond);

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

        let before_withdraw_unbonded = XAssets::pcx_free_balance(&1);
        assert_ok!(t_withdraw_unbonded(1, 0),);
        assert_eq!(XAssets::pcx_free_balance(&1), before_withdraw_unbonded + 5);
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

#[test]
fn staking_reward_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let t_1 = 67;
        let t_2 = 68;
        let t_3 = 69;

        assert_ok!(t_issue_pcx(t_1, 100));
        assert_ok!(t_issue_pcx(t_2, 100));
        assert_ok!(t_issue_pcx(t_3, 100));

        // 5_000_000_000
        //
        // |_vesting_account: 1_000_000_000
        // |_treasury_reward: 480_000_000   12%
        // |_mining_reward:   3_520_000_000 88%
        //   |__ Staking        90%
        //   |__ Asset Mining   10%
        //
        // When you start session 1, actually there are 3 session rounds.
        // the session reward has been minted 3 times.
        t_start_session(1);

        let sub_total = 4_000_000_000u128;

        let treasury_reward = sub_total * 12 / 100;
        let mining_reward = sub_total * 88 / 100;

        let staking_mining_reward = mining_reward * 90 / 100;
        let asset_mining_reward = mining_reward * 10 / 100;

        // (1, 10) => 10 / 100
        // (2, 20) => 20 / 100
        // (3, 30) => 30 / 100
        // (4, 40) => 40 / 100
        let total_staked = 100;
        let validators = vec![1, 2, 3, 4];

        let test_validator_reward =
            |validator: AccountId,
             initial_free: Balance,
             staked: Balance,
             session_index: SessionIndex| {
                let val_total_reward = staking_mining_reward * staked / total_staked;
                // 10% -> validator
                // 90% -> validator's reward pot
                assert_eq!(
                    XAssets::pcx_free_balance(&validator),
                    initial_free + val_total_reward * session_index as u128 / 10
                );
                assert_eq!(
                    XAssets::pcx_free_balance(
                        &DummyStakingRewardPotAccountDeterminer::reward_pot_account_for(&validator)
                    ),
                    0 + (val_total_reward - val_total_reward / 10) * session_index as u128
                );
            };

        test_validator_reward(1, 100 - 10, 10, 1);
        test_validator_reward(2, 200 - 20, 20, 1);
        test_validator_reward(3, 300 - 30, 30, 1);
        test_validator_reward(4, 400 - 40, 40, 1);

        assert_eq!(
            XAssets::pcx_free_balance(&TREASURY_ACCOUNT),
            (treasury_reward + asset_mining_reward) * 1
        );

        let validators_reward_pot = validators
            .iter()
            .map(DummyStakingRewardPotAccountDeterminer::reward_pot_account_for)
            .collect::<Vec<_>>();

        let issued_manually = 100 * 3;
        let endowed = 100 + 200 + 300 + 400;
        assert_eq!(
            XAssets::pcx_total_balance(),
            5_000_000_000u128 + issued_manually + endowed
        );

        let mut all = Vec::new();
        all.push(VESTING_ACCOUNT);
        all.push(TREASURY_ACCOUNT);
        all.extend_from_slice(&[t_1, t_2, t_3]);
        all.extend_from_slice(&validators);
        all.extend_from_slice(&validators_reward_pot);

        let total_issuance = || {
            all.iter()
                .map(|x| XAssets::all_type_asset_balance(x, &xpallet_protocol::PCX))
                .sum::<u128>()
        };

        assert_eq!(XAssets::pcx_total_balance(), total_issuance());

        t_start_session(2);
        assert_eq!(
            XAssets::pcx_total_balance(),
            5_000_000_000u128 * 2 + issued_manually + endowed
        );
    });
}

#[test]
fn staker_reward_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        // todo!("");
    });
}

#[test]
fn slash_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        // todo!("force_new_era_test");
    });
}
