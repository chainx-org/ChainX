// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::common::*;

#[test]
pub fn test_address() {
    XGatewayBitcoin::verify_btc_address(&b"mqVznxoxdeSNYgDCg6ZVE5pc6476BY6zHK".to_vec()).unwrap();
}

#[test]
fn test_accountid() {
    let _g = force_ss58_version();
    let script = Script::from(
        "5Uj3ehamDZWPfgA8iAZenhcAmPDakjf4aMbkBB4dXVvjoW6x@33"
            .as_bytes()
            .to_vec(),
    );
    let s = script.to_bytes();
    assert!(<Test as Trait>::AccountExtractor::extract_account(&s).is_some());
}
