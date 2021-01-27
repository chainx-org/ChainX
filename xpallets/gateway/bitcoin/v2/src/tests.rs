use frame_support::{assert_err, assert_ok, dispatch::DispatchResultWithPostInfo};

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

#[test]
fn test_issue() {
    use sp_core::U256;
    ExtBuilder::build(BuildConfig {
        exchange_price: 1000,
        exchange_decimal: 0,
        minimium_vault_collateral: 100,
        issue_griefing_fee: 10,
    })
    .execute_with(|| {
        assert_ok!(register_vault(3, 3000, "test"));

        assert_err!(
            issue::Pallet::<Test>::request_issue(Origin::signed(2), 1, 1, 200),
            vault::Error::<Test>::VaultNotFound
        );
        assert_err!(
            issue::Pallet::<Test>::request_issue(Origin::signed(2), 3, 1, 80),
            issue::Error::<Test>::InsufficientGriefingCollateral
        );

        assert_ok!(issue::Pallet::<Test>::request_issue(
            Origin::signed(2),
            3,
            1,
            200
        ));

        let issue_request = <issue::IssueRequests<Test>>::get(U256::one()).unwrap();
        assert_eq!(issue_request.vault, 3);
        assert_eq!(issue_request.griefing_collateral, 200);
        assert_eq!(issue_request.btc_address, "test".as_bytes().to_vec());
        assert_eq!(issue_request.requester, 2);

        let free_balance = pallet_balances::Module::<Test>::free_balance(2);
        assert_eq!(free_balance, 1800);
    })
}
