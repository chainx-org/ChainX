mod mock;

use mock::*;

#[test]
fn base() {
    ExtBuilder::default().build().execute_with(|| {})
}
