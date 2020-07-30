use crate::mock::*;
use crate::*;

use frame_support::assert_ok;
use xpallet_protocol::X_BTC;

#[test]
fn test_normal_case() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(
            XAssets::all_type_total_asset_balance(&X_BTC),
            100 + 200 + 300 + 400
        );

        assert_ok!(XAssets::transfer(
            Some(1).into(),
            999,
            X_BTC.into(),
            50_u128.into(),
            b"".to_vec().into()
        ));
        assert_eq!(XAssets::free_balance_of(&1, &X_BTC), 50);
        assert_eq!(XAssets::free_balance_of(&999, &X_BTC), 50);

        assert_eq!(
            XAssets::all_type_total_asset_balance(&X_BTC),
            100 + 200 + 300 + 400
        );

        assert_ok!(XAssets::move_balance(
            &X_BTC,
            &1,
            AssetType::Free,
            &999,
            AssetType::ReservedWithdrawal,
            25
        ));
        assert_eq!(
            XAssets::total_asset_balance_of(&X_BTC, AssetType::Free),
            1000 - 25
        );
        assert_eq!(
            XAssets::total_asset_balance_of(&X_BTC, AssetType::ReservedWithdrawal),
            25
        );

        assert_ok!(XAssets::destroy(&X_BTC, &999, 15));
        assert_eq!(
            XAssets::asset_type_balance(&999, &X_BTC, AssetType::ReservedWithdrawal),
            10
        );
        assert_eq!(
            XAssets::total_asset_balance_of(&X_BTC, AssetType::ReservedWithdrawal),
            10
        );
    })
}
