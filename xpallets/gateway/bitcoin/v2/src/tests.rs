use frame_support::{assert_err, assert_ok, dispatch::DispatchResultWithPostInfo};

use super::mock::{ExtBuilder, Origin, Test};
use super::vault::pallet::{Error, Pallet};

fn register_vault(id: u64, collateral: u128, addr: &str) -> DispatchResultWithPostInfo {
    Pallet::<Test>::register_vault(Origin::signed(id), collateral, addr.as_bytes().to_vec())
}

#[test]
fn test_register_vault() {
    ExtBuilder::build(100).execute_with(|| {
        assert_err!(
            register_vault(1, 10000, "test"),
            Error::<Test>::InsufficientFunds
        );
        assert_err!(
            register_vault(1, 10, "test"),
            Error::<Test>::InsufficientVaultCollateralAmount
        );
        assert_ok!(register_vault(1, 200, "test"));
        assert_err!(
            register_vault(1, 200, "testuu"),
            Error::<Test>::VaultAlreadyRegistered
        );
        assert_err!(
            register_vault(2, 200, "test"),
            Error::<Test>::BtcAddressOccupied
        );
    })
}

#[test]
fn test_add_extra_collateral() {
    ExtBuilder::build(100).execute_with(|| {
        assert_err!(
            Pallet::<Test>::add_extra_collateral(Origin::signed(1), 100),
            Error::<Test>::VaultNotFound
        );
        assert_ok!(register_vault(1, 200, "test"));
        assert_err!(
            Pallet::<Test>::add_extra_collateral(Origin::signed(1), 10000),
            Error::<Test>::InsufficientFunds
        );
        assert_ok!(Pallet::<Test>::add_extra_collateral(Origin::signed(1), 100));
        let free_balance = pallet_balances::Module::<Test>::free_balance(1);
        assert_eq!(free_balance, 700);
    })
}
