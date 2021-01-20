use super::mock::{ExtBuilder, Origin, Test};
use super::vault::pallet as vault;

#[test]
fn test_register_vault() {
    ExtBuilder::build().execute_with(|| {
        let ret: Result<_, _> = vault::Pallet::<Test>::register_vault(
            Origin::signed(1),
            2000,
            "test_address".as_bytes().to_vec(),
        );
        println!("{:?}", ret);
    })
}
