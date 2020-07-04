use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok};
use xpallet_protocol::X_BTC;

fn t_system_block_number_inc(number: BlockNumber) {
    System::set_block_number((System::block_number() + number).into());
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

fn t_xbtc_latest_total_weights() -> VoteWeight {
    <XMiningAsset as ComputeVoteWeight<AccountId>>::settle_claimee_weight(
        &X_BTC,
        System::block_number().saturated_into(),
    )
}

fn t_xbtc_move(from: AccountId, to: AccountId, value: Balance) {
    XAssets::move_balance(
        &X_BTC,
        &from,
        AssetType::Free,
        &to,
        AssetType::Free,
        value,
        true,
    )
    .unwrap();
}

fn t_xbtc_latest_weight_of(who: AccountId) -> VoteWeight {
    <XMiningAsset as ComputeVoteWeight<AccountId>>::settle_claimer_weight(
        &who,
        &X_BTC,
        System::block_number().saturated_into(),
    )
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
