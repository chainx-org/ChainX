mod utils;

use sp_arithmetic::Percent;

use frame_support::{assert_err, assert_ok, instances::Instance1};
use frame_system::RawOrigin;

use super::mock::*;
use crate::pallet;
use utils::*;

#[allow(non_upper_case_globals)]
const Alice: AccountId = 1;
#[allow(non_upper_case_globals)]
const Bob: AccountId = 2;
#[allow(non_upper_case_globals)]
const Solid: AccountId = 3;

#[test]
fn test_register_vault() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_err!(
            t_register_vault(Alice, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
        assert_err!(
            t_register_vault(Alice, 10, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            pallet::Error::<Test, Instance1>::CollateralAmountTooSmall
        );
        assert_ok!(t_register_vault(
            Alice,
            2000,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"
        ));
        assert_err!(
            t_register_vault(Alice, 2000, "3LrrqZ2LtZxAcroVaYKgM6yDeRszV2sY1r"),
            pallet::Error::<Test, Instance1>::VaultAlreadyRegistered
        );
        assert_err!(
            t_register_vault(Bob, 2000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            pallet::Error::<Test, Instance1>::BtcAddressOccupied
        );

        // Dogecoin
        assert_ok!(t_register_vault(
            Solid,
            2000,
            "np38J5RC9azJCtmTM3KNCzx99kguVps1X4"
        ));
    })
}

#[test]
#[allow(non_upper_case_globals)]
fn test_add_extra_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_err!(
            XGatewayBitcoin::add_extra_collateral(Origin::signed(Alice), 100),
            pallet::Error::<Test, Instance1>::VaultNotFound
        );
        assert_ok!(t_register_vault(
            Alice,
            2000,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"
        ));
        assert_err!(
            XGatewayBitcoin::add_extra_collateral(Origin::signed(Alice), 10000),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
        assert_ok!(XGatewayBitcoin::add_extra_collateral(
            Origin::signed(Alice),
            2000
        ));
        let free_balance = Balances::free_balance(Alice);
        assert_eq!(free_balance, 6000);
    })
}

#[test]
fn test_update_exchange_rate() {
    use crate::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        let exchange_rate = XGatewayBitcoin::exchange_rate();
        assert_eq!(exchange_rate.price, 1);
        assert_eq!(exchange_rate.decimal, 3);

        let new_exchange_rate = TradingPrice {
            price: 100,
            decimal: 10,
        };

        assert_err!(
            XGatewayBitcoin::update_exchange_rate(Origin::signed(Bob), new_exchange_rate.clone()),
            pallet::Error::<Test, Instance1>::NotOracle
        );
        assert_ok!(XGatewayBitcoin::force_update_oracles(
            Origin::root(),
            vec![Solid]
        ));
        assert_ok!(XGatewayBitcoin::update_exchange_rate(
            Origin::signed(Solid),
            new_exchange_rate.clone()
        ));
        let exchange_rate = XGatewayBitcoin::exchange_rate();
        assert_eq!(exchange_rate, new_exchange_rate);
    })
}

#[test]
fn test_bridge_needs_to_update_exchange_rate() {
    use crate::types::{ErrorCode, Status, TradingPrice};
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_eq!(XGatewayBitcoin::bridge_status(), Status::Running);
        XGatewayBitcoin::force_update_exchange_rate(
            RawOrigin::Root.into(),
            TradingPrice {
                price: 1u128,
                decimal: 3u8,
            },
        )
        .unwrap();
        assert_eq!(XGatewayBitcoin::bridge_status(), Status::Running);

        System::set_block_number(10000);
        run_to_block(10003);

        assert_eq!(
            XGatewayBitcoin::bridge_status(),
            Status::Error(ErrorCode::EXCHANGE_RATE_EXPIRED)
        );

        XGatewayBitcoin::force_update_exchange_rate(
            RawOrigin::Root.into(),
            TradingPrice {
                price: 1u128,
                decimal: 3u8,
            },
        )
        .unwrap();

        run_to_block(10004);
        assert_eq!(XGatewayBitcoin::bridge_status(), Status::Running);
    })
}

#[test]
fn test_match_vault() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(Alice, 10000, "3LrrqZ2LtZxAcroVaYKgM6yDeRszV2sY1r").unwrap();
        t_register_vault(Bob, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        let vault = XGatewayBitcoin::get_first_matched_vault(2);
        assert_eq!(vault.unwrap().0, Alice);
        XGatewayBitcoin::request_issue(Origin::signed(Solid), Alice, 3).unwrap();
        let vault = XGatewayBitcoin::get_first_matched_vault(3);
        assert_eq!(vault.unwrap().0, Bob);
    })
}

#[test]
fn test_issue_request() {
    use super::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(Solid, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        XGatewayBitcoin::update_issue_griefing_fee(Origin::root(), Percent::from_parts(10))
            .unwrap();
        XGatewayBitcoin::force_update_exchange_rate(
            Origin::root(),
            TradingPrice {
                price: 1,
                decimal: 2,
            },
        )
        .unwrap();

        assert_ok!(XGatewayBitcoin::request_issue(
            Origin::signed(Bob),
            Solid,
            1
        ));

        let reserved_balance = <<Test as xpallet_assets::Config>::Currency>::reserved_balance(2);
        assert_eq!(reserved_balance, 10);
        let issue_request = XGatewayBitcoin::try_get_issue_request(1).unwrap();
        assert_eq!(issue_request.griefing_collateral, 10);
        assert_eq!(issue_request.requester, 2);
        assert_eq!(issue_request.vault, 3);
        assert_eq!(issue_request.open_time, 0);

        // check vault's token status
        let vault = XGatewayBitcoin::try_get_vault(&issue_request.vault).unwrap();
        assert_eq!(vault.to_be_issued_tokens, issue_request.btc_amount);

        t_register_btc().unwrap();

        // execute issue_request
        assert_ok!(XGatewayBitcoin::execute_issue(
            Origin::signed(Alice),
            1,
            vec![],
            vec![],
            vec![],
        ));

        let user_xbtc = xpallet_assets::Module::<Test>::asset_balance_of(
            &issue_request.requester,
            &BridgeTargetAssetId::get(),
            xpallet_assets::AssetType::Usable,
        );
        assert_eq!(user_xbtc, 1);

        let vault = XGatewayBitcoin::try_get_vault(&issue_request.vault).unwrap();
        assert_eq!(
            XGatewayBitcoin::token_asset_of(&issue_request.vault),
            issue_request.btc_amount
        );
        assert_eq!(vault.to_be_issued_tokens, 0);

        assert_err!(
            XGatewayBitcoin::execute_issue(Origin::signed(Alice), 1, vec![], vec![], vec![],),
            pallet::Error::<Test, Instance1>::IssueRequestNotFound
        );
    })
}

#[test]
fn test_cancel_issue_request() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XGatewayBitcoin::update_issue_griefing_fee(RawOrigin::Root.into(), Percent::from_parts(10))
            .unwrap();
        t_register_vault(Solid, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        XGatewayBitcoin::request_issue(Origin::signed(Alice), Solid, 1).unwrap();

        System::set_block_number(5000);
        assert_err!(
            XGatewayBitcoin::cancel_issue(Origin::signed(Alice), 1),
            pallet::Error::<Test, Instance1>::IssueRequestNotExpired
        );

        System::set_block_number(10020);
        assert_ok!(XGatewayBitcoin::cancel_issue(Origin::signed(Alice), 1));

        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(Solid), 17000);
        assert_eq!(<pallet::CurrencyOf<Test>>::free_balance(Alice), 13000);
    })
}

// Basic function test cases.
#[test]
fn test_lock_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_ok!(XGatewayBitcoin::lock_collateral(&Alice, 200));
        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(Alice), 200);
        assert_err!(
            XGatewayBitcoin::lock_collateral(&Alice, 100_000),
            pallet_balances::Error::<Test>::InsufficientBalance,
        );
    });
}

#[test]
fn test_slash_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XGatewayBitcoin::lock_collateral(&Alice, 200).unwrap();
        assert_err!(
            XGatewayBitcoin::slash_vault(&Alice, &Bob, 300),
            pallet::Error::<Test, Instance1>::InsufficientCollateral
        );
        assert_ok!(XGatewayBitcoin::slash_vault(&Alice, &Bob, 200));
        assert_eq!(<pallet::CurrencyOf<Test>>::free_balance(Alice), 9800);
        assert_eq!(<pallet::CurrencyOf<Test>>::free_balance(Bob), 20200);
        assert_eq!(XGatewayBitcoin::total_collateral(), 0);
    });
}

#[test]
fn test_redeem_request_err_with_insufficiant_assets_funds() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(Solid, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        t_register_btc().unwrap();

        XAssets::issue(&BridgeTokenAssetId::get(), &Solid, 1).unwrap();
        XAssets::issue(&BridgeTargetAssetId::get(), &Bob, 1).unwrap();

        assert_err!(
            XGatewayBitcoin::request_redeem(
                Origin::signed(Bob),
                Solid,
                1000,
                "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec()
            ),
            pallet::Error::<Test, Instance1>::InsufficiantAssetsFunds
        );
    })
}

#[test]
fn test_redeem_request_ok() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(Solid, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        t_register_btc().unwrap();

        XAssets::issue(&BridgeTokenAssetId::get(), &Solid, 1).unwrap();
        XAssets::issue(&BridgeTargetAssetId::get(), &Bob, 1).unwrap();

        assert_ok!(XGatewayBitcoin::request_redeem(
            Origin::signed(Bob),
            Solid,
            1,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec()
        ));

        let vault = XGatewayBitcoin::try_get_vault(&Solid).unwrap();
        assert_eq!(vault.to_be_redeemed_tokens, 1);

        let redeem_request = pallet::RedeemRequests::<Test, Instance1>::get(&1).unwrap();
        assert_eq!(redeem_request.btc_amount, 1);

        let requester_locked_xbtc = xpallet_assets::Module::<Test>::asset_balance_of(
            &2,
            &BridgeTargetAssetId::get(),
            xpallet_assets::AssetType::ReservedWithdrawal,
        );
        assert_eq!(requester_locked_xbtc, 1);
    });
}

#[test]
fn test_redeem_execute() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(Solid, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        t_register_btc().unwrap();

        XAssets::issue(&BridgeTokenAssetId::get(), &Solid, 1).unwrap();
        XAssets::issue(&BridgeTargetAssetId::get(), &Bob, 1).unwrap();

        XGatewayBitcoin::request_redeem(
            Origin::signed(Bob),
            Solid,
            1,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec(),
        )
        .unwrap();

        assert_ok!(XGatewayBitcoin::execute_redeem(
            Origin::signed(Alice),
            1,
            vec![],
            vec![],
            vec![]
        ));

        // Request is removed.
        assert_eq!(pallet::RedeemRequests::<Test, Instance1>::get(&1), None);

        // Check requester assets after executing.
        let requester_locked_xbtc = xpallet_assets::Module::<Test>::asset_balance_of(
            &2,
            &BridgeTargetAssetId::get(),
            xpallet_assets::AssetType::ReservedWithdrawal,
        );
        assert_eq!(requester_locked_xbtc, 0);

        // Vault's to-be-redeem-tokens decreased.
        let vault = XGatewayBitcoin::try_get_vault(&Solid).unwrap();
        assert_eq!(vault.to_be_redeemed_tokens, 0);

        // Vault's issued_tokens decreased.
        assert_eq!(XGatewayBitcoin::token_asset_of(&Solid), 0);
    })
}

#[test]
fn test_calculate_required_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XGatewayBitcoin::update_issue_griefing_fee(Origin::root(), Percent::from_parts(10))
            .unwrap();
        assert_eq!(
            XGatewayBitcoin::calculate_required_collateral(100).unwrap(),
            10000
        );
    })
}

#[test]
fn test_calculate_collateral_ratio() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_ok!(XGatewayBitcoin::calculate_collateral_ratio(10, 40000));
    })
}
