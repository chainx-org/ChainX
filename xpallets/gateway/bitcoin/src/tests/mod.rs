// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

mod header;
mod trustee;
mod tx;

use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};

use xp_gateway_common::AccountExtractor;

use light_bitcoin::script::Script;

use crate::mock::{Test, XGatewayBitcoin};
use crate::Config;

#[test]
pub fn test_verify_btc_address() {
    let address = b"mqVznxoxdeSNYgDCg6ZVE5pc6476BY6zHK".to_vec();
    assert!(XGatewayBitcoin::verify_btc_address(&address).is_ok());
}

#[test]
fn test_account_ss58_version() {
    set_default_ss58_version(Ss58AddressFormat::ChainXAccount);
    let script = Script::from(
        "5Uj3ehamDZWPfgA8iAZenhcAmPDakjf4aMbkBB4dXVvjoW6x@33"
            .as_bytes()
            .to_vec(),
    );
    let data = script.to_bytes();
    assert!(<Test as Config>::AccountExtractor::extract_account(&data).is_some());
}
