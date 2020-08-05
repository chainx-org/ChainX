use super::*;
use crate::mock::*;
use frame_support::{
    assert_err, assert_ok,
    traits::{Get, OnInitialize},
};
use xp_mining_staking::SessionIndex;
use xpallet_protocol::X_BTC;

fn t_system_block_number_inc(number: BlockNumber) {
    System::set_block_number((System::block_number() + number).into());
}

fn t_bond(who: AccountId, target: AccountId, value: Balance) -> DispatchResult {
    XStaking::bond(Origin::signed(who), target, value, b"memo".as_ref().into())
}

fn t_issue_pcx(who: AccountId, value: Balance) {
    let _ = Balances::deposit_creating(&who, value);
}

fn t_issue_xbtc(to: AccountId, value: Balance) -> DispatchResult {
    XAssets::issue(&X_BTC, &to, value)
}

fn t_register_xbtc() -> DispatchResult {
    let btc_asset = crate::mock::btc();
    XAssets::register_asset(
        frame_system::RawOrigin::Root.into(),
        btc_asset.0,
        btc_asset.1,
        btc_asset.2,
        true,
        true,
    )
}

fn t_xbtc_total() -> Balance {
    XAssets::all_type_total_asset_balance(&X_BTC).saturated_into()
}

fn t_xbtc_latest_total_weights() -> WeightType {
    <XMiningAsset as ComputeMiningWeight<AccountId, BlockNumber>>::settle_claimee_weight(
        &X_BTC,
        System::block_number(),
    )
}

fn t_xbtc_move(from: AccountId, to: AccountId, value: Balance) {
    assert_ok!(XAssets::move_balance(
        &X_BTC,
        &from,
        AssetType::Free,
        &to,
        AssetType::Free,
        value
    ));
}

fn t_xbtc_latest_weight_of(who: AccountId) -> WeightType {
    <XMiningAsset as ComputeMiningWeight<AccountId, BlockNumber>>::settle_claimer_weight(
        &who,
        &X_BTC,
        System::block_number().saturated_into(),
    )
}

fn t_xbtc_set_claim_frequency_limit(new: BlockNumber) {
    assert_ok!(XMiningAsset::set_claim_frequency_limit(
        frame_system::RawOrigin::Root.into(),
        X_BTC,
        new
    ));
}

fn t_xbtc_set_claim_staking_requirement(new: StakingRequirement) {
    assert_ok!(XMiningAsset::set_claim_staking_requirement(
        frame_system::RawOrigin::Root.into(),
        X_BTC,
        new
    ));
}

fn assert_xbtc_reward_pot_balance(value: Balance) {
    assert_eq!(
        Balances::free_balance(
            &DummyAssetRewardPotAccountDeterminer::reward_pot_account_for(&xpallet_protocol::X_BTC)
        ),
        value
    );
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
fn on_register_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(MiningPrevilegedAssets::get(), vec![]);

        t_system_block_number_inc(1);

        assert_ok!(t_register_xbtc());
        assert_eq!(MiningPrevilegedAssets::get(), vec![1]);
        assert_eq!(
            <AssetLedgers<Test>>::get(1),
            AssetLedger {
                last_total_mining_weight: 0,
                last_total_mining_weight_update: 2,
            }
        );
        assert_eq!(t_xbtc_total(), 0);
    });
}

#[test]
fn mining_weights_should_work_when_moving_xbtc() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(MiningPrevilegedAssets::get(), vec![]);

        t_system_block_number_inc(1);

        assert_ok!(t_register_xbtc());
        assert_eq!(MiningPrevilegedAssets::get(), vec![1]);
        assert_eq!(
            <AssetLedgers<Test>>::get(1),
            AssetLedger {
                last_total_mining_weight: 0,
                last_total_mining_weight_update: 2,
            }
        );
        assert_eq!(t_xbtc_total(), 0);

        t_system_block_number_inc(1);

        let t_1 = 888;
        let t_2 = 999;

        assert_ok!(t_issue_xbtc(t_1, 100));

        assert_eq!(t_xbtc_total(), 100);
        assert_eq!(
            <AssetLedgers<Test>>::get(X_BTC),
            AssetLedger {
                last_total_mining_weight: 0,
                last_total_mining_weight_update: 3,
            }
        );
        assert_eq!(
            <MinerLedgers<Test>>::get(t_1, X_BTC),
            MinerLedger {
                last_mining_weight: 0,
                last_mining_weight_update: 3,
                last_claim: None
            }
        );

        t_system_block_number_inc(1);
        assert_ok!(t_issue_xbtc(t_2, 200));

        assert_eq!(t_xbtc_total(), 300);
        assert_eq!(
            <AssetLedgers<Test>>::get(X_BTC),
            AssetLedger {
                last_total_mining_weight: 100,
                last_total_mining_weight_update: 4,
            }
        );
        assert_eq!(
            <MinerLedgers<Test>>::get(t_1, X_BTC),
            MinerLedger {
                last_mining_weight: 0,
                last_mining_weight_update: 3,
                last_claim: None
            }
        );
        assert_eq!(
            <MinerLedgers<Test>>::get(t_2, X_BTC),
            MinerLedger {
                last_mining_weight: 0,
                last_mining_weight_update: 4,
                last_claim: None
            }
        );

        t_system_block_number_inc(1);
        assert_ok!(t_issue_xbtc(t_1, 100));
        assert_eq!(t_xbtc_total(), 400);
        assert_eq!(
            <AssetLedgers<Test>>::get(X_BTC),
            AssetLedger {
                last_total_mining_weight: 100 + 300 * 1,
                last_total_mining_weight_update: 5,
            }
        );
        assert_eq!(
            <MinerLedgers<Test>>::get(t_1, X_BTC),
            MinerLedger {
                last_mining_weight: 0 + 100 * 2,
                last_mining_weight_update: 5,
                last_claim: None
            }
        );

        t_system_block_number_inc(1);

        assert_eq!(
            t_xbtc_latest_total_weights(),
            vec![t_1, t_2]
                .into_iter()
                .map(|who| t_xbtc_latest_weight_of(who))
                .sum()
        );
    });
}

#[test]
fn sum_of_miner_weights_and_asset_total_weights_should_equal() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(MiningPrevilegedAssets::get(), vec![]);

        t_system_block_number_inc(1);

        assert_ok!(t_register_xbtc());

        t_system_block_number_inc(1);

        let t_1 = 777;
        let t_2 = 888;
        let t_3 = 999;

        assert_ok!(t_issue_xbtc(t_1, 100));

        t_system_block_number_inc(1);
        assert_ok!(t_issue_xbtc(t_2, 200));

        t_system_block_number_inc(1);
        assert_ok!(t_issue_xbtc(t_1, 100));

        t_system_block_number_inc(1);

        t_xbtc_move(t_1, t_2, 50);

        t_system_block_number_inc(1);

        t_xbtc_move(t_2, t_3, 30);
        t_xbtc_move(t_1, t_3, 10);

        t_system_block_number_inc(1);

        t_xbtc_move(t_3, t_1, 5);
        t_xbtc_move(t_1, t_3, 80);

        assert_eq!(
            t_xbtc_latest_total_weights(),
            vec![t_1, t_2, t_3]
                .into_iter()
                .map(|who| t_xbtc_latest_weight_of(who))
                .sum()
        );
    });
}

#[test]
fn claim_restriction_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(t_register_xbtc());
        let t_1 = 777;
        assert_ok!(t_issue_xbtc(t_1, 100));

        // Block 1
        t_start_session(1);
        t_xbtc_set_claim_frequency_limit(2);
        t_xbtc_set_claim_staking_requirement(0);

        // Block 2
        t_start_session(2);
        assert_ok!(XMiningAsset::claim(Origin::signed(t_1), X_BTC));

        // Block 3
        t_start_session(3);
        assert_err!(
            XMiningAsset::claim(Origin::signed(t_1), X_BTC),
            Error::<Test>::UnexpiredFrequencyLimit
        );

        // Block 4
        t_start_session(4);
        assert_err!(
            XMiningAsset::claim(Origin::signed(t_1), X_BTC),
            Error::<Test>::UnexpiredFrequencyLimit
        );

        // Block 5
        t_start_session(5);
        assert_ok!(XMiningAsset::claim(Origin::signed(t_1), X_BTC));

        // Block 6
        t_start_session(6);
        t_xbtc_set_claim_frequency_limit(0);
        t_xbtc_set_claim_staking_requirement(10);
        assert_err!(
            XMiningAsset::claim(Origin::signed(t_1), X_BTC),
            Error::<Test>::InsufficientStaking
        );

        // Block 7
        t_start_session(7);
        t_issue_pcx(1, 1_000_000_000_000u128);
        t_issue_pcx(t_1, 1_000_000_000_000u128);
        assert_ok!(t_bond(1, 1, 100_000_000_000));
        // total dividend: 704000000
        let total_mining_dividend = 704000000;
        // the claimer needs 10x dividend of Staking locked.
        assert_ok!(t_bond(t_1, 1, total_mining_dividend * 10 - 1));
        assert_err!(
            XMiningAsset::claim(Origin::signed(t_1), X_BTC),
            Error::<Test>::InsufficientStaking
        );

        assert_ok!(t_bond(t_1, 1, 1));
        assert_ok!(XMiningAsset::claim(Origin::signed(t_1), X_BTC));
    });
}

#[test]
fn total_issuance_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let validators = vec![1, 2, 3, 4];
        let validators_reward_pot = validators
            .iter()
            .map(DummyStakingRewardPotAccountDeterminer::reward_pot_account_for)
            .collect::<Vec<_>>();

        let mut all = Vec::new();
        all.extend_from_slice(&validators);
        all.extend_from_slice(&validators_reward_pot);
        all.push(VESTING_ACCOUNT);
        all.push(TREASURY_ACCOUNT);
        all.push(DummyAssetRewardPotAccountDeterminer::reward_pot_account_for(&X_BTC));

        let total_issuance = || all.iter().map(Balances::total_balance).sum::<u128>();

        let initial = 100 + 200 + 300 + 400;
        t_start_session(1);
        assert_eq!(total_issuance(), 5_000_000_000 + initial);

        t_start_session(2);
        assert_eq!(total_issuance(), 5_000_000_000 * 2 + initial);

        t_start_session(3);
        assert_eq!(total_issuance(), 5_000_000_000 * 3 + initial);
    });
}

fn t_set_xbtc_asset_power(new: FixedAssetPower) {
    assert_ok!(XMiningAsset::set_x_asset_power(
        Origin::root(),
        xpallet_protocol::X_BTC,
        new
    ));
}

#[test]
fn asset_mining_reward_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(t_register_xbtc());

        let t_1 = 666_666;
        assert_ok!(t_issue_xbtc(t_1, 1));

        t_issue_pcx(4, 1000);

        t_set_xbtc_asset_power(100);

        assert_ok!(t_bond(4, 4, 800));

        // total_staked: 100 + 800 = 900;
        //
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

        let asset_mining_reward = mining_reward * 10 / 100;

        //    ├──> Staking        3_168_000_000 90% 900
        //    └──> Asset Mining     352_000_000 10% 100

        assert_eq!(Balances::free_balance(&TREASURY_ACCOUNT), treasury_reward);
        assert_xbtc_reward_pot_balance(asset_mining_reward);

        // XBTC mining power too few.
        t_set_xbtc_asset_power(50);
        t_start_session(2);
        assert_eq!(
            Balances::free_balance(&TREASURY_ACCOUNT),
            treasury_reward * 2 + asset_mining_reward / 2 // extra treasury from asset mining reward.
        );
        assert_xbtc_reward_pot_balance(asset_mining_reward + asset_mining_reward / 2);

        // XBTC mining power too much.
        t_set_xbtc_asset_power(200);
        t_start_session(3);
        let treasury_balance = treasury_reward * 3 + asset_mining_reward / 2; // no extra split.
        assert_eq!(Balances::free_balance(&TREASURY_ACCOUNT), treasury_balance);
        let xbtc_pot_balance = asset_mining_reward + asset_mining_reward / 2 + asset_mining_reward;
        assert_xbtc_reward_pot_balance(xbtc_pot_balance);

        // Disable the staking requirement.
        assert_ok!(XMiningAsset::set_claim_staking_requirement(
            Origin::root(),
            xpallet_protocol::X_BTC,
            0
        ));

        assert_eq!(Balances::free_balance(&t_1), 0);
        assert_ok!(XMiningAsset::claim(
            Origin::signed(t_1),
            xpallet_protocol::X_BTC
        ));
        assert_eq!(
            Balances::free_balance(&t_1),
            xbtc_pot_balance - xbtc_pot_balance / 10
        );
        let referral = DummyGatewayReferralGetter::referral_of(&t_1, xpallet_protocol::X_BTC)
            .unwrap_or(TREASURY_ACCOUNT);
        assert_eq!(Balances::free_balance(&referral), xbtc_pot_balance / 10);
        assert_eq!(Balances::free_balance(&TREASURY_ACCOUNT), treasury_balance);
    });
}
