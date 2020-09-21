// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};

fn t_issue_pcx(to: AccountId, value: Balance) {
    XStaking::mint(&to, value);
}

fn t_register(who: AccountId, initial_bond: Balance) -> DispatchResult {
    let mut referral_id = who.to_string().as_bytes().to_vec();

    if referral_id.len() < 2 {
        referral_id.extend_from_slice(&[0, 0, 0, who as u8]);
    }

    XStaking::register(Origin::signed(who), referral_id, initial_bond)
}

fn t_bond(who: AccountId, target: AccountId, value: Balance) -> DispatchResult {
    XStaking::bond(Origin::signed(who), target, value)
}

fn t_rebond(who: AccountId, from: AccountId, to: AccountId, value: Balance) -> DispatchResult {
    XStaking::rebond(Origin::signed(who), from, to, value)
}

fn t_unbond(who: AccountId, target: AccountId, value: Balance) -> DispatchResult {
    XStaking::unbond(Origin::signed(who), target, value)
}

fn t_withdraw_unbonded(
    who: AccountId,
    target: AccountId,
    unbonded_index: UnbondedIndex,
) -> DispatchResult {
    XStaking::unlock_unbonded_withdrawal(Origin::signed(who), target, unbonded_index)
}

fn t_system_block_number_inc(number: BlockNumber) {
    System::set_block_number((System::block_number() + number).into());
}

fn t_make_a_validator_candidate(who: AccountId, self_bonded: Balance) {
    t_issue_pcx(who, self_bonded);
    assert_ok!(t_register(who, self_bonded));
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

fn assert_bonded_locks(who: AccountId, value: Balance) {
    assert_eq!(
        *<Locks<Test>>::get(who)
            .entry(LockedType::Bonded)
            .or_default(),
        value
    );
}

fn assert_bonded_withdrawal_locks(who: AccountId, value: Balance) {
    assert_eq!(
        *<Locks<Test>>::get(who)
            .entry(LockedType::BondedWithdrawal)
            .or_default(),
        value
    );
}

#[test]
fn cannot_force_chill_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        t_make_a_validator_candidate(123, 100);
        assert_eq!(XStaking::can_force_chilled(), true);
        assert_ok!(XStaking::chill(Origin::signed(123)));
        assert_ok!(XStaking::chill(Origin::signed(2)));
        assert_ok!(XStaking::chill(Origin::signed(3)));
        assert_ok!(XStaking::chill(Origin::signed(4)));
        assert_err!(
            XStaking::chill(Origin::signed(1)),
            <Error<Test>>::TooFewActiveValidators
        );
        t_make_a_validator_candidate(1234, 100);
        assert_ok!(XStaking::chill(Origin::signed(1)));
    });
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

        let before_bond = Balances::usable_balance(&1);
        // old_lock 10
        let old_lock = *<Locks<Test>>::get(1).get(&LockedType::Bonded).unwrap();
        assert_ok!(t_bond(1, 2, 10));

        assert_bonded_locks(1, old_lock + 10);
        assert_eq!(Balances::usable_balance(&1), before_bond - 10);
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
                unbonded_chunks: vec![]
            }
        );
    });
}

#[test]
fn unbond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_err!(t_unbond(1, 2, 50), Error::<Test>::InvalidUnbondBalance);

        assert_bonded_locks(1, 10);
        t_system_block_number_inc(1);

        assert_ok!(t_bond(1, 2, 10));
        assert_bonded_locks(1, 10 + 10);

        t_system_block_number_inc(1);

        assert_ok!(t_unbond(1, 2, 5));
        assert_bonded_locks(1, 10 + 10 - 5);
        assert_bonded_withdrawal_locks(1, 5);

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
            XStaking::unbond(Origin::signed(1), 2, 50),
            Error::<Test>::InvalidUnbondBalance
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
                unbonded_chunks: vec![]
            }
        );

        assert_eq!(
            <Nominations<Test>>::get(1, 3),
            NominatorLedger {
                nomination: 5,
                last_vote_weight: 0,
                last_vote_weight_update: 3,
                unbonded_chunks: vec![]
            }
        );

        assert_eq!(<LastRebondOf<Test>>::get(1), Some(3));

        // Block 4
        t_system_block_number_inc(1);
        assert_err!(t_rebond(1, 2, 3, 3), Error::<Test>::NoMoreRebond);

        // The rebond operation is limited to once per bonding duration.
        assert_ok!(XStaking::set_bonding_duration(Origin::root(), 2));

        t_system_block_number_inc(1);
        assert_err!(t_rebond(1, 2, 3, 3), Error::<Test>::NoMoreRebond);

        t_system_block_number_inc(1);
        assert_ok!(t_rebond(1, 2, 3, 3));
    });
}

#[test]
fn withdraw_unbond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        t_system_block_number_inc(1);

        let before_bond = Balances::usable_balance(&1);
        assert_ok!(t_bond(1, 2, 10));
        assert_eq!(Balances::usable_balance(&1), before_bond - 10);

        t_system_block_number_inc(1);

        assert_ok!(t_unbond(1, 2, 5));
        let before_unbond = Balances::usable_balance(&1);
        assert_eq!(Balances::usable_balance(&1), before_unbond);

        assert_eq!(
            <Nominations<Test>>::get(1, 2).unbonded_chunks,
            vec![Unbonded {
                value: 5,
                locked_until: DEFAULT_BONDING_DURATION + 3
            }]
        );

        t_system_block_number_inc(DEFAULT_BONDING_DURATION);
        assert_err!(
            t_withdraw_unbonded(1, 2, 0),
            Error::<Test>::UnbondedWithdrawalNotYetDue
        );

        t_system_block_number_inc(1);

        let before_withdraw_unbonded = Balances::usable_balance(&1);
        assert_ok!(t_withdraw_unbonded(1, 2, 0),);
        assert_eq!(Balances::usable_balance(&1), before_withdraw_unbonded + 5);
        assert_bonded_withdrawal_locks(1, 0);
    });
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

        t_issue_pcx(t_1, 100);
        t_issue_pcx(t_2, 100);
        t_issue_pcx(t_3, 100);

        // Total minted per session:
        // 5_000_000_000
        // │
        // ├──> vesting_account:  1_000_000_000
        // ├──> treasury_reward:    480_000_000 12% <--------
        // └──> mining_reward:    3_520_000_000 88%          |
        //    │                                              |
        //    ├──> Staking        3_168_000_000 90%          |
        //    └──> Asset Mining     352_000_000 10% ---------
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
                    Balances::free_balance(&validator),
                    initial_free + val_total_reward * session_index as u128 / 10
                );
                assert_eq!(
                    Balances::free_balance(
                        &DummyStakingRewardPotAccountDeterminer::reward_pot_account_for(&validator)
                    ),
                    0 + (val_total_reward - val_total_reward / 10) * session_index as u128
                );
            };

        test_validator_reward(1, 100, 10, 1);
        test_validator_reward(2, 200, 20, 1);
        test_validator_reward(3, 300, 30, 1);
        test_validator_reward(4, 400, 40, 1);

        assert_eq!(
            Balances::free_balance(&TREASURY_ACCOUNT),
            (treasury_reward + asset_mining_reward) * 1
        );

        let validators_reward_pot = validators
            .iter()
            .map(DummyStakingRewardPotAccountDeterminer::reward_pot_account_for)
            .collect::<Vec<_>>();

        let issued_manually = 100 * 3;
        let endowed = 100 + 200 + 300 + 400;
        assert_eq!(
            Balances::total_issuance(),
            5_000_000_000u128 + issued_manually + endowed
        );

        let mut all = Vec::new();
        all.push(VESTING_ACCOUNT);
        all.push(TREASURY_ACCOUNT);
        all.extend_from_slice(&[t_1, t_2, t_3]);
        all.extend_from_slice(&validators);
        all.extend_from_slice(&validators_reward_pot);

        let total_issuance = || all.iter().map(|x| Balances::free_balance(x)).sum::<u128>();

        assert_eq!(Balances::total_issuance(), total_issuance());

        t_start_session(2);
        assert_eq!(
            Balances::total_issuance(),
            5_000_000_000u128 * 2 + issued_manually + endowed
        );
    });
}

fn t_reward_pot_balance(validator: AccountId) -> Balance {
    XStaking::free_balance(
        &DummyStakingRewardPotAccountDeterminer::reward_pot_account_for(&validator),
    )
}

#[test]
fn staker_reward_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let t_1 = 1111;
        let t_2 = 2222;
        let t_3 = 3333;

        t_issue_pcx(t_1, 100);
        t_issue_pcx(t_2, 100);
        t_issue_pcx(t_3, 100);

        assert_eq!(
            <ValidatorLedgers<Test>>::get(1),
            ValidatorLedger {
                total: 10,
                last_total_vote_weight: 0,
                last_total_vote_weight_update: 0,
            }
        );
        assert_ok!(t_bond(t_1, 1, 10));
        assert_eq!(
            <Nominations<Test>>::get(t_1, 1),
            NominatorLedger {
                nomination: 10,
                last_vote_weight: 0,
                last_vote_weight_update: 1,
                unbonded_chunks: vec![]
            }
        );
        assert_eq!(
            <ValidatorLedgers<Test>>::get(1),
            ValidatorLedger {
                total: 20,
                last_total_vote_weight: 10,
                last_total_vote_weight_update: 1,
            }
        );

        const TOTAL_STAKING_REWARD: Balance = 3_168_000_000;

        let calc_reward_for_pot =
            |validator_votes: Balance, total_staked: Balance, total_reward: Balance| {
                let total_reward_for_validator = validator_votes * total_reward / total_staked;
                let to_validator = total_reward_for_validator / 10;
                let to_pot = total_reward_for_validator - to_validator;
                to_pot
            };

        // Block 1
        // total_staked = val(10+10) + val2(20) + val(30) + val(40) = 110
        // reward pot:
        // 1: 3_168_000_000 * 20/110 * 90% = 51_840_000
        // 2: 3_168_000_000 * 20/110 * 90% = 51_840_000
        // 3: 3_168_000_000 * 30/110 * 90% = 777_600_000
        // 4: 3_168_000_000 * 40/110 * 90% = 1_036_800_000
        t_start_session(1);
        assert_eq!(t_reward_pot_balance(1), 518_400_000);
        assert_eq!(t_reward_pot_balance(2), 518_400_000);
        assert_eq!(t_reward_pot_balance(3), 777_600_000);
        assert_eq!(t_reward_pot_balance(4), 1_036_800_000);

        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 20,
                last_total_vote_weight: 0,
                last_total_vote_weight_update: 0,
            }
        );
        assert_ok!(t_bond(t_2, 2, 20));
        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 20 + 20,
                last_total_vote_weight: 20,
                last_total_vote_weight_update: 1,
            }
        );

        // Block 2
        // total_staked = val(10+10) + val2(20+20) + val(30) + val(40) = 130
        // reward pot:
        // There might be a calculation loss using 90% directly, the actual
        // calculation is:
        // validator 3: 3_168_000_000 * 30/130 = 731076923
        //                    |_ validator 3: 731076923 / 10 = 73107692
        //                    |_ validator 3's reward pot: 731076923 - 73107692

        t_start_session(2);
        // The order is [3, 4, 1, 2] when calculating.
        assert_eq!(t_reward_pot_balance(3), 777_600_000 + 731076923 - 73107692);
        assert_eq!(
            t_reward_pot_balance(3),
            777_600_000 + calc_reward_for_pot(30, 130, TOTAL_STAKING_REWARD)
        );
        assert_eq!(t_reward_pot_balance(4), 1914092307);
        assert_eq!(t_reward_pot_balance(1), 957046154);
        assert_eq!(t_reward_pot_balance(2), 1395692309);

        // validator 1: vote weight = 10 + 20 * 1 = 30
        // t_1 vote weight: 10 * 1  = 10
        assert_ok!(XStaking::claim(Origin::signed(t_1), 1));
        // t_1 = reward_pot_balance * 10 / 30
        assert_eq!(XStaking::free_balance(&t_1), 100 + 957046154 / 3);

        // validator 2: vote weight = 40 * 1 + 20 = 60
        // t_2 vote weight = 20 * 1 = 20
        assert_ok!(XStaking::claim(Origin::signed(t_2), 2));
        assert_eq!(XStaking::free_balance(&t_2), 100 + 1395692309 * 20 / 60);

        assert_ok!(XStaking::set_minimum_validator_count(Origin::root(), 3));
        assert_ok!(XStaking::chill(Origin::signed(3)));

        // Block 3
        t_start_session(3);
        // validator 3 is chilled now, not rewards then.
        assert_eq!(
            t_reward_pot_balance(3),
            777_600_000 + calc_reward_for_pot(30, 130, TOTAL_STAKING_REWARD)
        );
    });
}

#[test]
fn slash_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        // todo!("force_new_era_test");
    });
}

#[test]
fn mint_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(Balances::total_issuance(), 1000);
        let to_mint = 666;
        XStaking::mint(&7777, to_mint);
        assert_eq!(Balances::total_issuance(), 1000 + to_mint);
        assert_eq!(Balances::free_balance(&7777), to_mint);
    });
}

#[test]
fn balances_reserve_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let who = 7777;
        let to_mint = 10;
        XStaking::mint(&who, to_mint);
        assert_eq!(Balances::free_balance(&who), 10);

        // Bond 6
        assert_ok!(XStaking::bond_reserve(&who, 6));
        assert_eq!(Balances::usable_balance(&who), 4);
        let mut locks = BTreeMap::new();
        locks.insert(LockedType::Bonded, 6);
        assert_eq!(XStaking::locks(&who), locks);
        assert_eq!(
            frame_system::Account::<Test>::get(&who).data,
            pallet_balances::AccountData {
                free: 10,
                reserved: 0,
                misc_frozen: 6,
                fee_frozen: 6
            }
        );
        assert_err!(
            Balances::transfer(Some(who).into(), 6, 1000),
            pallet_balances::Error::<Test, _>::InsufficientBalance
        );

        // Bond 2 extra
        assert_ok!(XStaking::bond_reserve(&who, 2));
        let mut locks = BTreeMap::new();
        locks.insert(LockedType::Bonded, 8);
        assert_eq!(
            frame_system::Account::<Test>::get(&who).data,
            pallet_balances::AccountData {
                free: 10,
                reserved: 0,
                misc_frozen: 8,
                fee_frozen: 8
            }
        );
        assert_err!(
            XStaking::bond_reserve(&who, 3),
            <Error<Test>>::InsufficientBalance
        );

        // Unbond 5 now, the frozen balances stay the same,
        // only internal Staking locked state changes.
        assert_ok!(XStaking::unbond_reserve(&who, 5));
        let mut locks = BTreeMap::new();
        locks.insert(LockedType::Bonded, 3);
        locks.insert(LockedType::BondedWithdrawal, 5);
        assert_eq!(XStaking::locks(&who), locks);
        assert_eq!(
            frame_system::Account::<Test>::get(&who).data,
            pallet_balances::AccountData {
                free: 10,
                reserved: 0,
                misc_frozen: 8,
                fee_frozen: 8
            }
        );

        // Unlock unbonded withdrawal 4.
        XStaking::apply_unlock_unbonded_withdrawal(&who, 4);
        let mut locks = BTreeMap::new();
        locks.insert(LockedType::Bonded, 3);
        locks.insert(LockedType::BondedWithdrawal, 1);
        assert_eq!(XStaking::locks(&who), locks);
        assert_eq!(
            frame_system::Account::<Test>::get(&who).data,
            pallet_balances::AccountData {
                free: 10,
                reserved: 0,
                misc_frozen: 4,
                fee_frozen: 4
            }
        );

        // Unlock unbonded withdrawal 1.
        XStaking::apply_unlock_unbonded_withdrawal(&who, 1);
        let mut locks = BTreeMap::new();
        locks.insert(LockedType::Bonded, 3);
        assert_eq!(XStaking::locks(&who), locks);
        assert_eq!(
            frame_system::Account::<Test>::get(&who).data,
            pallet_balances::AccountData {
                free: 10,
                reserved: 0,
                misc_frozen: 3,
                fee_frozen: 3
            }
        );
    });
}

#[test]
fn referral_id_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(XStaking::register(
            Origin::signed(111),
            b"referral1".to_vec(),
            0
        ));
        assert_err!(
            XStaking::register(Origin::signed(112), b"referral1".to_vec(), 0),
            Error::<Test>::OccupiedReferralIdentity
        );

        assert_ok!(XStaking::register(
            Origin::signed(112),
            b"referral2".to_vec(),
            0
        ));
    });
}

#[test]
fn migration_session_offset_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let test_cases = vec![
            (MigrationSessionOffset::get(), INITIAL_REWARD),
            (MigrationSessionOffset::get() + 1, INITIAL_REWARD / 2),
            (
                MigrationSessionOffset::get() + SESSIONS_PER_ROUND,
                INITIAL_REWARD / 2,
            ),
            (
                MigrationSessionOffset::get() + SESSIONS_PER_ROUND + 1,
                INITIAL_REWARD / 4,
            ),
            (
                MigrationSessionOffset::get() + SESSIONS_PER_ROUND * 2,
                INITIAL_REWARD / 4,
            ),
            (
                MigrationSessionOffset::get() + SESSIONS_PER_ROUND * 2 + 1,
                INITIAL_REWARD / 8,
            ),
        ];

        for (session_index, session_reward) in test_cases {
            let session_reward = session_reward as Balance;
            assert_eq!(XStaking::this_session_reward(session_index), session_reward);

            if session_reward == INITIAL_REWARD as Balance {
                assert_eq!(
                    XStaking::try_vesting(session_index, session_reward),
                    session_reward * 4 / 5
                );
            } else {
                assert_eq!(
                    XStaking::try_vesting(session_index, session_reward),
                    session_reward
                );
            }
        }
    });
}
