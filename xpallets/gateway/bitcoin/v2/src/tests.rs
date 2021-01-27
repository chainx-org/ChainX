use frame_support::{assert_err, assert_ok, dispatch::DispatchResultWithPostInfo};

use super::assets::pallet as assets;
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
fn test_btc_to_pcx() {
    ExtBuilder::build(BuildConfig {
        exchange_price: 123123123,
        exchange_decimal: 6,
        ..Default::default()
    })
    .execute_with(|| {
        assert_eq!(
            assets::Pallet::<Test>::btc_to_pcx(100_000_000).unwrap(),
            12_312_312_300
        );
        assert_eq!(
            assets::Pallet::<Test>::pcx_to_btc(12_312_312_300).unwrap(),
            100_000_000
        );
    })
}
