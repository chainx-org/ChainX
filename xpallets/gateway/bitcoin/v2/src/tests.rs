use light_bitcoin::chain::Transaction;

use sp_arithmetic::Percent;

use frame_support::traits::Hooks;
use frame_support::{
    assert_err, assert_ok,
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
};
use frame_system::RawOrigin;

use super::assets::pallet as assets;
use super::issue::pallet as issue;
use super::redeem::pallet as redeem;
use super::vault::pallet as vault;

use super::mock::*;

fn t_register_vault(id: u64, collateral: u128, addr: &str) -> DispatchResultWithPostInfo {
    Vault::register_vault(Origin::signed(id), collateral, addr.as_bytes().to_vec())
}

fn run_to_block(index: u64) {
    while System::block_number() < index {
        Redeem::on_finalize(System::block_number());
        Assets::on_finalize(System::block_number());
        Vault::on_finalize(System::block_number());
        Issue::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::set_block_number(System::block_number() + 1);

        System::on_initialize(System::block_number());
        Issue::on_initialize(System::block_number());
        Vault::on_initialize(System::block_number());
        Assets::on_initialize(System::block_number());
        Redeem::on_initialize(System::block_number());
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

// ============================================================
// test vault
// ============================================================

#[test]
fn test_register_vault() {
    ExtBuilder::build(BuildConfig {
        minimium_vault_collateral: 100,
        ..Default::default()
    })
    .execute_with(|| {
        assert_err!(
            t_register_vault(1, 10000, "test"),
            assets::Error::<Test>::InsufficientFunds
        );
        assert_err!(
            t_register_vault(1, 10, "test"),
            vault::Error::<Test>::InsufficientVaultCollateralAmount
        );
        assert_ok!(t_register_vault(1, 200, "test"));
        assert_err!(
            t_register_vault(1, 200, "testuu"),
            vault::Error::<Test>::VaultAlreadyRegistered
        );
        assert_err!(
            t_register_vault(2, 200, "test"),
            vault::Error::<Test>::BtcAddressOccupied
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
            Vault::add_extra_collateral(Origin::signed(1), 100),
            vault::Error::<Test>::VaultNotFound
        );
        assert_ok!(t_register_vault(1, 200, "test"));
        assert_err!(
            Vault::add_extra_collateral(Origin::signed(1), 10000),
            assets::Error::<Test>::InsufficientFunds
        );
        assert_ok!(Vault::add_extra_collateral(Origin::signed(1), 100));
        let free_balance = Balances::free_balance(1);
        assert_eq!(free_balance, 700);
    })
}

// ============================================================
// test assets
// ============================================================

#[test]
fn test_update_exchange_rate() {
    use super::assets::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        let exchange_rate = Assets::exchange_rate();
        assert_eq!(exchange_rate.price, 0);
        assert_eq!(exchange_rate.decimal, 0);

        let new_exchange_rate = TradingPrice {
            price: 100,
            decimal: 10,
        };

        assert_err!(
            Assets::update_exchange_rate(Origin::signed(2), new_exchange_rate.clone()),
            assets::Error::<Test>::OperationForbidden
        );
        assert_ok!(Assets::force_update_oracles(Origin::root(), vec![0]));
        assert_ok!(Assets::update_exchange_rate(
            Origin::signed(0),
            new_exchange_rate.clone()
        ));
        let exchange_rate = Assets::exchange_rate();
        assert_eq!(exchange_rate, new_exchange_rate);
    })
}

#[test]
fn test_bridge_needs_to_update_exchange_rate() {
    use crate::assets::types::{ErrorCode, Status, TradingPrice};
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_eq!(Assets::bridge_status(), Status::Running);

        assets::ExchangeRateExpiredPeriod::<Test>::put(2);
        Assets::force_update_exchange_rate(
            RawOrigin::Root.into(),
            TradingPrice {
                price: 1u128,
                decimal: 3u8,
            },
        )
        .unwrap();
        assert_eq!(Assets::bridge_status(), Status::Running);

        run_to_block(3);

        assert_eq!(
            Assets::bridge_status(),
            Status::Error(ErrorCode::EXCHANGE_RATE_EXPIRED)
        );

        Assets::force_update_exchange_rate(
            RawOrigin::Root.into(),
            TradingPrice {
                price: 1u128,
                decimal: 3u8,
            },
        )
        .unwrap();

        run_to_block(4);
        assert_eq!(Assets::bridge_status(), Status::Running);
    })
}

// ============================================================
// test assets
// ============================================================

#[test]
fn test_issue_request() {
    use super::assets::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(1, 800, "test").unwrap();
        Issue::update_expired_time(Origin::root(), 10u64).unwrap();
        Issue::update_griefing_fee(Origin::root(), Percent::from_parts(10)).unwrap();
        Assets::force_update_exchange_rate(
            Origin::root(),
            TradingPrice {
                price: 1,
                decimal: 2,
            },
        )
        .unwrap();
        assert_err!(
            Issue::request_issue(Origin::signed(2), 1, 1, 2),
            issue::Error::<Test>::InsufficientGriefingCollateral
        );
        assert_ok!(Issue::request_issue(Origin::signed(2), 1, 1, 100));
        let reserved_balance = <<Test as xpallet_assets::Config>::Currency>::reserved_balance(2);
        assert_eq!(reserved_balance, 100);
        let issue_request = Issue::get_issue_request_by_id(1).unwrap();
        assert_eq!(issue_request.griefing_collateral, 100);
        assert_eq!(issue_request.requester, 2);
        assert_eq!(issue_request.vault, 1);
        assert_eq!(issue_request.open_time, 0);

        // check vault's token status
        let vault = Vault::get_vault_by_id(&issue_request.vault).unwrap();
        assert_eq!(vault.to_be_issued_tokens, issue_request.btc_amount);

        t_register_btc().unwrap();

        // execute issue_request
        assert_ok!(Issue::execute_issue(
            Origin::signed(1),
            1,
            vec![],
            vec![],
            Transaction::default(),
        ));
        let vault = Vault::get_vault_by_id(&issue_request.vault).unwrap();
        assert_eq!(vault.issued_tokens, issue_request.btc_amount);
        assert_eq!(vault.to_be_issued_tokens, 0);
    })
}

// Basic function test cases.
#[test]
fn test_lock_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_ok!(Assets::lock_collateral(&1, 200));
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(1), 200);
        assert_err!(
            Assets::lock_collateral(&1, 1000),
            assets::Error::<Test>::InsufficientFunds
        );
    });
}

#[test]
fn test_slash_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        Assets::lock_collateral(&1, 200).unwrap();
        assert_err!(
            Assets::slash_collateral(&1, &2, 300),
            assets::Error::<Test>::InsufficientCollateral
        );
        assert_ok!(Assets::slash_collateral(&1, &2, 200));
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(1), 0);
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(2), 200);
    });
}

#[test]
fn test_release_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        Assets::lock_collateral(&1, 200).unwrap();
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(1), 200);
        assert_ok!(Assets::release_collateral(&1, 200));
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(1), 0);
        assert_err!(
            assets::Pallet::<Test>::release_collateral(&1, 200),
            assets::Error::<Test>::InsufficientCollateral
        );
    })
}

#[test]
fn test_redeem_request() {
    use super::assets::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(1, 800, "test").unwrap();
        Issue::update_expired_time(Origin::root(), 10u64).unwrap();
        Issue::update_griefing_fee(Origin::root(), Percent::from_parts(10)).unwrap();
        assets::Pallet::<Test>::force_update_exchange_rate(
            Origin::root(),
            TradingPrice {
                price: 1,
                decimal: 2,
            },
        )
        .unwrap();

        assert_ok!(issue::Pallet::<Test>::request_issue(
            Origin::signed(2),
            1,
            1,
            100
        ));

        let reserved_balance = <<Test as xpallet_assets::Config>::Currency>::reserved_balance(2);
        assert_eq!(reserved_balance, 100);

        assert_err!(
            redeem::Pallet::<Test>::request_redeem(
                Origin::signed(2),
                1,
                1000,
                "test".as_bytes().to_vec()
            ),
            redeem::Error::<Test>::InsufficiantAssetsFunds
        );

        assert_err!(
            redeem::Pallet::<Test>::request_redeem(
                Origin::signed(2),
                1,
                1,
                "test".as_bytes().to_vec(),
            ),
            redeem::Error::<Test>::InsufficiantAssetsFunds
        );
    })
}

#[test]
fn test_liquidation_redeem() {
    use super::assets::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        t_register_vault(1, 800, "test").unwrap();
        Issue::update_expired_time(Origin::root(), 10u64).unwrap();
        Issue::update_griefing_fee(Origin::root(), Percent::from_parts(10)).unwrap();
        assets::Pallet::<Test>::force_update_exchange_rate(
            Origin::root(),
            TradingPrice {
                price: 1,
                decimal: 2,
            },
        )
        .unwrap();

        assert_ok!(Issue::request_issue(Origin::signed(2), 1, 1, 100));

        let reserved_balance = <<Test as xpallet_assets::Config>::Currency>::reserved_balance(2);
        assert_eq!(reserved_balance, 100);

        assert_err!(
            redeem::Pallet::<Test>::liquidation_redeem(Origin::signed(2), 1,),
            redeem::Error::<Test>::InsufficiantAssetsFunds
        );
    })
}
