use sp_arithmetic::Percent;

use frame_support::traits::Hooks;
use frame_support::{
    assert_err, assert_ok,
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
};
use frame_system::RawOrigin;

use crate::types::RedeemRequestStatus;

use crate::pallet as xbridge;

use super::mock::*;

fn t_register_vault(id: u64, collateral: u128, addr: &str) -> DispatchResultWithPostInfo {
    XBridge::register_vault(Origin::signed(id), collateral, addr.as_bytes().to_vec())
}

fn run_to_block(index: u64) {
    while System::block_number() < index {
        XBridge::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::set_block_number(System::block_number() + 1);

        System::on_initialize(System::block_number());
        XBridge::on_initialize(System::block_number());
    }
}

fn t_register_btc() -> DispatchResult {
    type XAssetsRegistrar = xpallet_assets_registrar::Module<Test>;
    type XAssets = xpallet_assets::Module<Test>;
    let btc_asset = (
        xp_protocol::X_BTC,
        xpallet_assets_registrar::AssetInfo::new::<Test>(
            b"X-BTC".to_vec(),
            b"X-BTC".to_vec(),
            xpallet_assets_registrar::Chain::Bitcoin,
            8,
            b"ChainX's cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        xpallet_assets::AssetRestrictions::empty(),
    );
    XAssetsRegistrar::register(RawOrigin::Root.into(), btc_asset.0, btc_asset.1, true, true)?;
    XAssets::set_asset_limit(RawOrigin::Root.into(), btc_asset.0, btc_asset.2)
}

#[test]
fn test_register_vault() {
    ExtBuilder::build(BuildConfig {
        minimium_vault_collateral: 100,
        ..Default::default()
    })
    .execute_with(|| {
        assert_err!(
            t_register_vault(1, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            xbridge::Error::<Test>::InsufficientFunds
        );
        assert_err!(
            t_register_vault(1, 10, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            xbridge::Error::<Test>::InsufficientVaultCollateralAmount
        );
        assert_ok!(t_register_vault(
            1,
            200,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"
        ));
        assert_err!(
            t_register_vault(1, 200, "3LrrqZ2LtZxAcroVaYKgM6yDeRszV2sY1r"),
            xbridge::Error::<Test>::VaultAlreadyRegistered
        );
        assert_err!(
            t_register_vault(2, 200, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            xbridge::Error::<Test>::BtcAddressOccupied
        );
    })
}

#[test]
fn test_add_extra_collateral() {
    ExtBuilder::build(BuildConfig {
        minimium_vault_collateral: 100,
        ..Default::default()
    })
    .execute_with(|| {
        assert_err!(
            XBridge::add_extra_collateral(Origin::signed(1), 100),
            xbridge::Error::<Test>::VaultNotFound
        );
        assert_ok!(t_register_vault(
            1,
            200,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"
        ));
        assert_err!(
            XBridge::add_extra_collateral(Origin::signed(1), 10000),
            xbridge::Error::<Test>::InsufficientFunds
        );
        assert_ok!(XBridge::add_extra_collateral(Origin::signed(1), 100));
        let free_balance = Balances::free_balance(1);
        assert_eq!(free_balance, 9700);
    })
}

#[test]
fn test_update_exchange_rate() {
    use crate::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        let exchange_rate = XBridge::exchange_rate();
        assert_eq!(exchange_rate.price, 1);
        assert_eq!(exchange_rate.decimal, 3);

        let new_exchange_rate = TradingPrice {
            price: 100,
            decimal: 10,
        };

        assert_err!(
            XBridge::update_exchange_rate(Origin::signed(2), new_exchange_rate.clone()),
            xbridge::Error::<Test>::OperationForbidden
        );
        assert_ok!(XBridge::force_update_oracles(Origin::root(), vec![0]));
        assert_ok!(XBridge::update_exchange_rate(
            Origin::signed(0),
            new_exchange_rate.clone()
        ));
        let exchange_rate = XBridge::exchange_rate();
        assert_eq!(exchange_rate, new_exchange_rate);
    })
}

#[test]
fn test_bridge_needs_to_update_exchange_rate() {
    use crate::types::{ErrorCode, Status, TradingPrice};
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_eq!(XBridge::bridge_status(), Status::Running);

        xbridge::ExchangeRateExpiredPeriod::<Test>::put(2);
        XBridge::force_update_exchange_rate(
            RawOrigin::Root.into(),
            TradingPrice {
                price: 1u128,
                decimal: 3u8,
            },
        )
        .unwrap();
        assert_eq!(XBridge::bridge_status(), Status::Running);

        run_to_block(3);

        assert_eq!(
            XBridge::bridge_status(),
            Status::Error(ErrorCode::EXCHANGE_RATE_EXPIRED)
        );

        XBridge::force_update_exchange_rate(
            RawOrigin::Root.into(),
            TradingPrice {
                price: 1u128,
                decimal: 3u8,
            },
        )
        .unwrap();

        run_to_block(4);
        assert_eq!(XBridge::bridge_status(), Status::Running);
    })
}

#[test]
fn test_issue_request() {
    use super::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(3, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        XBridge::update_issue_expired_time(Origin::root(), 10u64).unwrap();
        XBridge::update_issue_griefing_fee(Origin::root(), Percent::from_parts(10)).unwrap();
        XBridge::force_update_exchange_rate(
            Origin::root(),
            TradingPrice {
                price: 1,
                decimal: 2,
            },
        )
        .unwrap();

        // request
        assert_err!(
            XBridge::request_issue(Origin::signed(2), 3, 1, 2),
            xbridge::Error::<Test>::InsufficientGriefingCollateral
        );
        assert_ok!(XBridge::request_issue(Origin::signed(2), 3, 1, 300));

        let reserved_balance = <<Test as xpallet_assets::Config>::Currency>::reserved_balance(2);
        assert_eq!(reserved_balance, 300);
        let issue_request = XBridge::get_issue_request_by_id(1).unwrap();
        assert_eq!(issue_request.griefing_collateral, 300);
        assert_eq!(issue_request.requester, 2);
        assert_eq!(issue_request.vault, 3);
        assert_eq!(issue_request.open_time, 0);

        // check vault's token status
        let vault = XBridge::get_vault_by_id(&issue_request.vault).unwrap();
        assert_eq!(vault.to_be_issued_tokens, issue_request.btc_amount);

        t_register_btc().unwrap();

        // execute issue_request
        assert_ok!(XBridge::execute_issue(
            Origin::signed(1),
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

        let vault = XBridge::get_vault_by_id(&issue_request.vault).unwrap();
        assert_eq!(vault.issued_tokens, issue_request.btc_amount);
        assert_eq!(vault.to_be_issued_tokens, 0);

        assert_err!(
            XBridge::execute_issue(Origin::signed(1), 1, vec![], vec![], vec![],),
            xbridge::Error::<Test>::IssueRequestDealt
        );
    })
}

#[test]
fn test_cancel_issue_request() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XBridge::update_issue_griefing_fee(RawOrigin::Root.into(), Percent::from_parts(10))
            .unwrap();
        XBridge::update_issue_expired_time(RawOrigin::Root.into(), 10).unwrap();

        t_register_vault(3, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        XBridge::request_issue(Origin::signed(1), 3, 1, 100).unwrap();

        System::set_block_number(5);
        assert_err!(
            XBridge::cancel_issue(Origin::signed(1), 1),
            xbridge::Error::<Test>::IssueRequestNotExpired
        );

        System::set_block_number(20);
        assert_ok!(XBridge::cancel_issue(Origin::signed(1), 1));

        assert_eq!(<xbridge::CurrencyOf<Test>>::reserved_balance(3), 17000);
        assert_eq!(<xbridge::CurrencyOf<Test>>::reserved_balance(1), 3000);
        assert_eq!(Balances::free_balance(1), 10000);
    })
}

// Basic function test cases.
#[test]
fn test_lock_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_ok!(XBridge::lock_collateral(&1, 200));
        assert_eq!(<xbridge::CurrencyOf<Test>>::reserved_balance(1), 200);
        assert_err!(
            XBridge::lock_collateral(&1, 100_000),
            xbridge::Error::<Test>::InsufficientFunds
        );
    });
}

#[test]
fn test_slash_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XBridge::lock_collateral(&1, 200).unwrap();
        assert_err!(
            XBridge::slash_collateral(&1, &2, 300),
            xbridge::Error::<Test>::InsufficientCollateral
        );
        assert_ok!(XBridge::slash_collateral(&1, &2, 200));
        assert_eq!(<xbridge::CurrencyOf<Test>>::reserved_balance(1), 0);
        assert_eq!(<xbridge::CurrencyOf<Test>>::reserved_balance(2), 200);
    });
}

#[test]
fn test_release_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XBridge::lock_collateral(&1, 200).unwrap();
        assert_eq!(<xbridge::CurrencyOf<Test>>::reserved_balance(1), 200);
        assert_ok!(XBridge::release_collateral(&1, 200));
        assert_eq!(<xbridge::CurrencyOf<Test>>::reserved_balance(1), 0);
        assert_err!(
            xbridge::Pallet::<Test>::release_collateral(&1, 200),
            xbridge::Error::<Test>::InsufficientCollateral
        );
    })
}

#[test]
fn test_redeem_request() {
    use super::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(3, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        XBridge::update_redeem_expired_time(Origin::root(), 10u64).unwrap();
        XBridge::update_issue_griefing_fee(Origin::root(), Percent::from_parts(10)).unwrap();
        XBridge::update_issue_expired_time(Origin::root(), 10u64).unwrap();

        XBridge::force_update_exchange_rate(
            Origin::root(),
            TradingPrice {
                price: 1,
                decimal: 2,
            },
        )
        .unwrap();

        XBridge::request_issue(Origin::signed(2), 3, 1, 100).unwrap();
        assert_err!(
            XBridge::request_redeem(
                Origin::signed(2),
                3,
                1000,
                "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec()
            ),
            xbridge::Error::<Test>::InsufficiantAssetsFunds
        );

        t_register_btc().unwrap();
        XBridge::execute_issue(Origin::signed(2), 1, vec![], vec![], vec![]).unwrap();

        // request redeem
        assert_ok!(XBridge::request_redeem(
            Origin::signed(2),
            3,
            1,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec()
        ));

        let vault = XBridge::get_vault_by_id(&3).unwrap();
        assert_eq!(vault.to_be_redeemed_tokens, 1);

        let redeem_request = xbridge::RedeemRequests::<Test>::get(&1).unwrap();
        assert_eq!(redeem_request.amount, 1);
        assert_eq!(redeem_request.status, RedeemRequestStatus::Processing);

        let requester_locked_xbtc = xpallet_assets::Module::<Test>::asset_balance_of(
            &2,
            &BridgeTargetAssetId::get(),
            xpallet_assets::AssetType::ReservedWithdrawal,
        );
        assert_eq!(requester_locked_xbtc, 1);

        assert_ok!(XBridge::execute_redeem(
            Origin::signed(1),
            1,
            vec![],
            vec![],
            vec![]
        ));

        let redeem_request = xbridge::RedeemRequests::<Test>::get(&1).unwrap();
        assert_eq!(redeem_request.amount, 1);
        assert_eq!(redeem_request.status, RedeemRequestStatus::Completed);

        // check requester assets after executing
        let requester_locked_xbtc = xpallet_assets::Module::<Test>::asset_balance_of(
            &2,
            &BridgeTargetAssetId::get(),
            xpallet_assets::AssetType::Locked,
        );
        assert_eq!(requester_locked_xbtc, 0);

        let vault = XBridge::get_vault_by_id(&3).unwrap();
        assert_eq!(vault.to_be_redeemed_tokens, 0);
        assert_eq!(vault.issued_tokens, 0);
    })
}

#[test]
fn test_calculate_required_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XBridge::update_issue_griefing_fee(Origin::root(), Percent::from_parts(10)).unwrap();
        assert_eq!(XBridge::calculate_required_collateral(100).unwrap(), 10000);
    })
}

#[test]
fn test_calculate_collateral_ratio() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_ok!(XBridge::calculate_collateral_ratio(10, 40000));
    })
}
