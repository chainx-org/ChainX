use frame_support::{assert_err, assert_ok};

use super::mock::{ExtBuilder, Origin, Test};
use super::vault::pallet as vault;

#[test]
fn test_register_vault() {
    ExtBuilder::build(100).execute_with(|| {
        assert_err!(
            vault::Pallet::<Test>::register_vault(
                Origin::signed(1),
                10000,
                "test".as_bytes().to_vec()
            ),
            vault::Error::<Test>::InsufficientFunds
        );
        assert_err!(
            vault::Pallet::<Test>::register_vault(
                Origin::signed(1),
                10,
                "test".as_bytes().to_vec()
            ),
            vault::Error::<Test>::InsufficientVaultCollateralAmount
        );
        assert_ok!(vault::Pallet::<Test>::register_vault(
            Origin::signed(1),
            200,
            "test".as_bytes().to_vec()
        ));
        assert_err!(
            vault::Pallet::<Test>::register_vault(
                Origin::signed(1),
                200,
                "testuu".as_bytes().to_vec()
            ),
            vault::Error::<Test>::VaultAlreadyRegistered
        );
        assert_err!(
            vault::Pallet::<Test>::register_vault(
                Origin::signed(2),
                200,
                "test".as_bytes().to_vec()
            ),
            vault::Error::<Test>::BtcAddressOccupied
        );
    })
}
