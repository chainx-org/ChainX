use sp_arithmetic::Percent;

use frame_support::traits::Hooks;
use frame_support::{
    assert_err, assert_ok,
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
};
use frame_system::RawOrigin;

use crate::pallet;

use super::mock::*;

fn t_register_vault(id: u64, collateral: u128, addr: &str) -> DispatchResultWithPostInfo {
    XGatewayBitcoin::register_vault(Origin::signed(id), collateral, addr.as_bytes().to_vec())
}

fn run_to_block(index: u64) {
    while System::block_number() < index {
        XGatewayBitcoin::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::set_block_number(System::block_number() + 1);

        System::on_initialize(System::block_number());
        XGatewayBitcoin::on_initialize(System::block_number());
    }
}

fn t_register_btc() -> DispatchResult {
    type XAssetsRegistrar = xpallet_assets_registrar::Module<Test>;
    type XAssets = xpallet_assets::Module<Test>;
    let assets = vec![
        (
            xp_protocol::X_BTC,
            xpallet_assets_registrar::AssetInfo::new::<Test>(
                b"X-BTC".to_vec(),
                b"X-BTC".to_vec(),
                xpallet_assets_registrar::Chain::Bitcoin,
                xp_protocol::BTC_DECIMALS,
                b"ChainX's cross-chain Bitcoin".to_vec(),
            )
            .unwrap(),
            xpallet_assets::AssetRestrictions::empty(),
        ),
        (
            xp_protocol::C_BTC,
            xpallet_assets_registrar::AssetInfo::new::<Test>(
                b"E-BTC".to_vec(),
                b"E-BTC".to_vec(),
                xpallet_assets_registrar::Chain::Bitcoin,
                xp_protocol::BTC_DECIMALS,
                b"ChainX's cross-chain Bitcoin".to_vec(),
            )
            .unwrap(),
            xpallet_assets::AssetRestrictions::empty(),
        ),
        (
            xp_protocol::S_BTC,
            xpallet_assets_registrar::AssetInfo::new::<Test>(
                b"S-BTC".to_vec(),
                b"S-BTC".to_vec(),
                xpallet_assets_registrar::Chain::Bitcoin,
                xp_protocol::BTC_DECIMALS,
                b"ChainX's cross-chain Bitcoin".to_vec(),
            )
            .unwrap(),
            xpallet_assets::AssetRestrictions::empty(),
        ),
    ];

    for (id, info, restrictions) in assets.into_iter() {
        XAssetsRegistrar::register(RawOrigin::Root.into(), id, info, true, true)?;
        XAssets::set_asset_limit(RawOrigin::Root.into(), id, restrictions)?;
    }
    Ok(())
}

#[test]
#[allow(non_upper_case_globals)]
fn test_register_vault() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        const Alice: AccountId = 1;
        const Bob: AccountId = 2;
        assert_err!(
            t_register_vault(Alice, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            pallet::Error::<Test>::InsufficientFunds
        );
        assert_err!(
            t_register_vault(Alice, 10, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            pallet::Error::<Test>::CollateralAmountTooSmall
        );
        assert_ok!(t_register_vault(
            Alice,
            2000,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"
        ));
        assert_err!(
            t_register_vault(Alice, 2000, "3LrrqZ2LtZxAcroVaYKgM6yDeRszV2sY1r"),
            pallet::Error::<Test>::VaultAlreadyRegistered
        );
        assert_err!(
            t_register_vault(Bob, 2000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"),
            pallet::Error::<Test>::BtcAddressOccupied
        );
    })
}

#[test]
#[allow(non_upper_case_globals)]
fn test_add_extra_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        const Alice: AccountId = 1;
        assert_err!(
            XGatewayBitcoin::add_extra_collateral(Origin::signed(Alice), 100),
            pallet::Error::<Test>::VaultNotFound
        );
        assert_ok!(t_register_vault(
            Alice,
            2000,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna"
        ));
        assert_err!(
            XGatewayBitcoin::add_extra_collateral(Origin::signed(Alice), 10000),
            pallet::Error::<Test>::InsufficientFunds
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
            XGatewayBitcoin::update_exchange_rate(Origin::signed(2), new_exchange_rate.clone()),
            pallet::Error::<Test>::NotOracle
        );
        assert_ok!(XGatewayBitcoin::force_update_oracles(
            Origin::root(),
            vec![0]
        ));
        assert_ok!(XGatewayBitcoin::update_exchange_rate(
            Origin::signed(0),
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
        const Alice: AccountId = 1u64;
        const Bob: AccountId = 2u64;
        t_register_vault(Alice, 10000, "3LrrqZ2LtZxAcroVaYKgM6yDeRszV2sY1r").unwrap();
        t_register_vault(Bob, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        let vault = XGatewayBitcoin::get_first_matched_vault(2);
        assert_eq!(vault.unwrap().0, Alice);
        XGatewayBitcoin::request_issue(Origin::signed(3), Alice, 3).unwrap();
        let vault = XGatewayBitcoin::get_first_matched_vault(3);
        assert_eq!(vault.unwrap().0, Bob);
    })
}

#[test]
fn test_issue_request() {
    use super::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(3, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
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

        assert_ok!(XGatewayBitcoin::request_issue(Origin::signed(2), 3, 1));

        let reserved_balance = <<Test as xpallet_assets::Config>::Currency>::reserved_balance(2);
        assert_eq!(reserved_balance, 10);
        let issue_request = XGatewayBitcoin::get_issue_request_by_id(1).unwrap();
        assert_eq!(issue_request.griefing_collateral, 10);
        assert_eq!(issue_request.requester, 2);
        assert_eq!(issue_request.vault, 3);
        assert_eq!(issue_request.open_time, 0);

        // check vault's token status
        let vault = XGatewayBitcoin::get_vault_by_id(&issue_request.vault).unwrap();
        assert_eq!(vault.to_be_issued_tokens, issue_request.btc_amount);

        t_register_btc().unwrap();

        // execute issue_request
        assert_ok!(XGatewayBitcoin::execute_issue(
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

        let vault = XGatewayBitcoin::get_vault_by_id(&issue_request.vault).unwrap();
        assert_eq!(
            XGatewayBitcoin::issued_tokens_of(&issue_request.vault),
            issue_request.btc_amount
        );
        assert_eq!(vault.to_be_issued_tokens, 0);

        assert_err!(
            XGatewayBitcoin::execute_issue(Origin::signed(1), 1, vec![], vec![], vec![],),
            pallet::Error::<Test>::IssueRequestNotFound
        );
    })
}

#[test]
fn test_cancel_issue_request() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XGatewayBitcoin::update_issue_griefing_fee(RawOrigin::Root.into(), Percent::from_parts(10))
            .unwrap();
        t_register_vault(3, 20000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        XGatewayBitcoin::request_issue(Origin::signed(1), 3, 1).unwrap();

        System::set_block_number(5000);
        assert_err!(
            XGatewayBitcoin::cancel_issue(Origin::signed(1), 1),
            pallet::Error::<Test>::IssueRequestNotExpired
        );

        System::set_block_number(10020);
        assert_ok!(XGatewayBitcoin::cancel_issue(Origin::signed(1), 1));

        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(3), 17000);
        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(1), 3000);
        assert_eq!(Balances::free_balance(1), 10000);
    })
}

// Basic function test cases.
#[test]
fn test_lock_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_ok!(XGatewayBitcoin::lock_collateral(&1, 200));
        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(1), 200);
        assert_err!(
            XGatewayBitcoin::lock_collateral(&1, 100_000),
            pallet::Error::<Test>::InsufficientFunds
        );
    });
}

#[test]
fn test_slash_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XGatewayBitcoin::lock_collateral(&1, 200).unwrap();
        assert_err!(
            XGatewayBitcoin::slash_collateral(&1, &2, 300),
            pallet::Error::<Test>::InsufficientCollateral
        );
        assert_ok!(XGatewayBitcoin::slash_collateral(&1, &2, 200));
        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(1), 0);
        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(2), 200);
    });
}

#[test]
fn test_release_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        XGatewayBitcoin::lock_collateral(&1, 200).unwrap();
        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(1), 200);
        assert_ok!(XGatewayBitcoin::unlock_collateral(&1, 200));
        assert_eq!(<pallet::CurrencyOf<Test>>::reserved_balance(1), 0);
        assert_err!(
            pallet::Pallet::<Test>::unlock_collateral(&1, 200),
            pallet::Error::<Test>::InsufficientCollateral
        );
    })
}

#[test]
fn test_redeem_request_err_with_insufficiant_assets_funds() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(3, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        t_register_btc().unwrap();

        XAssets::issue(&BridgeTokenAssetId::get(), &3, 1).unwrap();
        XAssets::issue(&BridgeTargetAssetId::get(), &2, 1).unwrap();

        assert_err!(
            XGatewayBitcoin::request_redeem(
                Origin::signed(2),
                3,
                1000,
                "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec()
            ),
            pallet::Error::<Test>::InsufficiantAssetsFunds
        );
    })
}

#[test]
fn test_redeem_request_ok() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(3, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        t_register_btc().unwrap();

        XAssets::issue(&BridgeTokenAssetId::get(), &3, 1).unwrap();
        XAssets::issue(&BridgeTargetAssetId::get(), &2, 1).unwrap();

        assert_ok!(XGatewayBitcoin::request_redeem(
            Origin::signed(2),
            3,
            1,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec()
        ));

        let vault = XGatewayBitcoin::get_vault_by_id(&3).unwrap();
        assert_eq!(vault.to_be_redeemed_tokens, 1);

        let redeem_request = pallet::RedeemRequests::<Test>::get(&1).unwrap();
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
        t_register_vault(3, 30000, "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna").unwrap();
        t_register_btc().unwrap();

        XAssets::issue(&BridgeTokenAssetId::get(), &3, 1).unwrap();
        XAssets::issue(&BridgeTargetAssetId::get(), &2, 1).unwrap();

        XGatewayBitcoin::request_redeem(
            Origin::signed(2),
            3,
            1,
            "16meyfSoQV6twkAAxPe51RtMVz7PGRmWna".as_bytes().to_vec(),
        )
        .unwrap();

        assert_ok!(XGatewayBitcoin::execute_redeem(
            Origin::signed(1),
            1,
            vec![],
            vec![],
            vec![]
        ));

        // Request is removed.
        assert_eq!(pallet::RedeemRequests::<Test>::get(&1), None);

        // Check requester assets after executing.
        let requester_locked_xbtc = xpallet_assets::Module::<Test>::asset_balance_of(
            &2,
            &BridgeTargetAssetId::get(),
            xpallet_assets::AssetType::ReservedWithdrawal,
        );
        assert_eq!(requester_locked_xbtc, 0);

        // Vault's to-be-redeem-tokens decreased.
        let vault = XGatewayBitcoin::get_vault_by_id(&3).unwrap();
        assert_eq!(vault.to_be_redeemed_tokens, 0);

        // Vault's issued_tokens decreased.
        assert_eq!(XGatewayBitcoin::issued_tokens_of(&3), 0);
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
