use sp_arithmetic::Percent;

use frame_support::{assert_err, assert_ok, dispatch::DispatchResultWithPostInfo};

use crate::assets::types::TradingPrice;

use super::assets::pallet as assets;
use super::issue::pallet as issue;
use super::mock::{BuildConfig, ExtBuilder, Origin, Test};
use super::vault::pallet as vault;

fn register_vault(id: u64, collateral: u128, addr: &str) -> DispatchResultWithPostInfo {
    vault::Pallet::<Test>::register_vault(Origin::signed(id), collateral, addr.as_bytes().to_vec())
}

#[test]
fn test_register_vault() {
    ExtBuilder::build(BuildConfig {
        minimium_vault_collateral: 100,
        ..Default::default()
    })
    .execute_with(|| {
        assert_err!(
            register_vault(1, 10000, "test"),
            assets::Error::<Test>::InsufficientFunds
        );
        assert_err!(
            register_vault(1, 10, "test"),
            vault::Error::<Test>::InsufficientVaultCollateralAmount
        );
        assert_ok!(register_vault(1, 200, "test"));
        assert_err!(
            register_vault(1, 200, "testuu"),
            vault::Error::<Test>::VaultAlreadyRegistered
        );
        assert_err!(
            register_vault(2, 200, "test"),
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
            vault::Pallet::<Test>::add_extra_collateral(Origin::signed(1), 100),
            vault::Error::<Test>::VaultNotFound
        );
        assert_ok!(register_vault(1, 200, "test"));
        assert_err!(
            vault::Pallet::<Test>::add_extra_collateral(Origin::signed(1), 10000),
            assets::Error::<Test>::InsufficientFunds
        );
        assert_ok!(vault::Pallet::<Test>::add_extra_collateral(
            Origin::signed(1),
            100
        ));
        let free_balance = pallet_balances::Module::<Test>::free_balance(1);
        assert_eq!(free_balance, 700);
    })
}

#[test]
fn test_update_exchange_rate() {
    use super::assets::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        let exchange_rate = assets::Pallet::<Test>::exchange_rate();
        assert_eq!(exchange_rate.price, 0);
        assert_eq!(exchange_rate.decimal, 0);

        let new_exchange_rate = TradingPrice {
            price: 100,
            decimal: 10,
        };

        assert_err!(
            assets::Pallet::<Test>::update_exchange_rate(
                Origin::signed(2),
                new_exchange_rate.clone()
            ),
            assets::Error::<Test>::OperationForbidden
        );
        assert_ok!(assets::Pallet::<Test>::force_update_oracles(
            Origin::root(),
            vec![0]
        ));
        assert_ok!(assets::Pallet::<Test>::update_exchange_rate(
            Origin::signed(0),
            new_exchange_rate.clone()
        ));
        let exchange_rate = assets::Pallet::<Test>::exchange_rate();
        assert_eq!(exchange_rate, new_exchange_rate);
    })
}

#[test]
fn test_issue_request() {
    use super::assets::types::TradingPrice;
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        register_vault(1, 800, "test").unwrap();
        issue::Pallet::<Test>::update_expired_time(Origin::root(), 10u64).unwrap();
        issue::Pallet::<Test>::update_griefing_fee(Origin::root(), Percent::from_parts(10))
            .unwrap();
        assets::Pallet::<Test>::force_update_exchange_rate(
            Origin::root(),
            TradingPrice {
                price: 1,
                decimal: 2,
            },
        )
        .unwrap();
        assert_err!(
            issue::Pallet::<Test>::request_issue(Origin::signed(2), 1, 1, 2),
            issue::Error::<Test>::InsufficientGriefingCollateral
        );
        assert_ok!(issue::Pallet::<Test>::request_issue(
            Origin::signed(2),
            1,
            1,
            100
        ));
        let reserved_balance = <<Test as xpallet_assets::Config>::Currency>::reserved_balance(2);
        assert_eq!(reserved_balance, 100);
        let issue_request = issue::Pallet::<Test>::get_issue_request_by_id(1).unwrap();
        assert_eq!(issue_request.griefing_collateral, 100);
        assert_eq!(issue_request.requester, 2);
        assert_eq!(issue_request.vault, 1);
        assert_eq!(issue_request.open_time, 0);

        // check vault's token status
        let vault = vault::Pallet::<Test>::get_vault_by_id(&issue_request.vault).unwrap();
        assert_eq!(vault.to_be_issued_tokens, issue_request.btc_amount);
    })
}

#[test]
fn test_lock_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assert_ok!(assets::Pallet::<Test>::lock_collateral(&1, 200));
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(1), 200);
        assert_err!(
            assets::Pallet::<Test>::lock_collateral(&1, 1000),
            assets::Error::<Test>::InsufficientFunds
        );
    });
}

#[test]
fn test_slash_collateral() {
    ExtBuilder::build(BuildConfig::default()).execute_with(|| {
        assets::Pallet::<Test>::lock_collateral(&1, 200).unwrap();
        assert_err!(
            assets::Pallet::<Test>::slash_collateral(&1, &2, 300),
            assets::Error::<Test>::InsufficientCollateral
        );
        assert_ok!(assets::Pallet::<Test>::slash_collateral(&1, &2, 200));
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(1), 0);
        assert_eq!(<assets::CurrencyOf<Test>>::reserved_balance(2), 200);
    });
}
