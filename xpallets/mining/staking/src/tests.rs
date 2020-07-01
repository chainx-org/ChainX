use super::*;
use crate::mock::*;

#[test]
fn bond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        println!(
            "xxxxxxxxxxxxxxxxxxx--- XAssets::asset_online:{:?}",
            XAssets::asset_online(0)
        );
        println!(
            "xxxxxxxxxxxxxxxxxxx--- XAssets::asset_online_test:{:?}",
            XAssets::asset_online_test(0)
        );
        println!(
            "{:?}",
            XStaking::bond(Origin::signed(1), 2, 5, b"memo".as_ref().into())
        );
    });
}
