use frame_support::{assert_err, assert_ok};

use super::mock::{ExtBuilder, Origin, Test};
use super::vault::pallet::{Error, Pallet};

#[test]
fn test_register_vault() {
    ExtBuilder::build(100).execute_with(|| {
        let register_vault = |id, collateral, addr: &str| {
            Pallet::<Test>::register_vault(Origin::signed(id), collateral, addr.as_bytes().to_vec())
        };
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
