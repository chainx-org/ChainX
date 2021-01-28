// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

// use frame_support::assert_ok;
use sp_core::offchain::{testing, OffchainExt};
use sp_io::TestExternalities;
// use sp_keystore::{
//     testing::KeyStore,
//     {KeystoreExt, SyncCryptoStore},
// };

use light_bitcoin::{
    keys::Network as BtcNetwork,
    primitives::{h256, hash_rev},
};

use crate::mock::XGatewayBitcoinRelay;
use light_bitcoin::chain::Transaction;
use light_bitcoin::keys::{Address, Network};
use light_bitcoin::script::Script;
use sp_core::crypto::{set_default_ss58_version, AccountId32, Ss58AddressFormat};
use sp_std::str::FromStr;
use xp_gateway_bitcoin::{
    extract_opreturn_data, AccountExtractor, BtcTxTypeDetector, OpReturnExtractor,
};
use xp_gateway_common::from_ss58_check;

#[test]
fn extract_account() {
    set_default_ss58_version(Ss58AddressFormat::SubstrateAccount);

    const DEPOSIT_HOT_ADDR: &str = "2N5QAjp4oaUbJCQqhsMiwSK1oYGJNUnAgqM";
    const DEPOSIT_COLD_ADDR: &str = "2N2AL9SfiGRssLt2bry6fnE4ruStLF7DtHD";
    let btc_tx_detector = BtcTxTypeDetector::new(
        Network::Testnet,
        0,
        (
            DEPOSIT_HOT_ADDR.parse::<Address>().unwrap(),
            DEPOSIT_COLD_ADDR.parse::<Address>().unwrap(),
        ),
        None,
    );

    let tx = "01000000021f3ffe48b4259a03792a48393028826fb8e5073bdaa68d9c3bd0b6c2c9bcad30010000006b483045022100b455f93c5b93a80255d4823ad5785b3aa6ab59cce03e045896b73f0f343ae1c702201de5d31272056168d81cf877313b033a5c47d643b38e6f36b4725f35929efa080121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d000000002a4d0feeb9c4a7901b3c0ebceb00f7fdfeba55a4528204deddbda9202bc0d08a010000006a47304402203b17fd6d2ceef10e56e7a28d46ce0937c88cdb8096f9807803199b7185159d4c02202c933afa661e333e5a648a1867b728bf63ee2ee91cc0117ad868ee1984703e710121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d0000000003605fa9000000000017a91485528a5e98cfb732129ff4a6b0d4d398c7be343687d4c91902000000001976a9147e836b50820d909dc10448ba7306a0f5dc6c755188ac0000000000000000326a303545577453636e65347a57734761503467566f38446d4c7043685678334d7a6f5154704b4a434564425459444131447900000000".parse::<Transaction>().unwrap();
    let result = (
        Some((
            "5EWtScne4zWsGaP4gVo8DmLpChVx3MzoQTpKJCEdBTYDA1Dy"
                .parse::<AccountId32>()
                .unwrap(),
            None,
        )),
        11100000,
    );

    let (result, value) = result;
    let (op_return, deposit_value) =
        btc_tx_detector.parse_deposit_transaction_outputs(&tx, OpReturnExtractor::extract_account);
    assert_eq!(op_return, result);
    assert_eq!(deposit_value, value);

    let mut account_info = None;
    for opreturn_script in tx
        .outputs
        .iter()
        .map(|output| Script::new(output.script_pubkey.clone()))
        .filter(|script| script.is_null_data_script())
    {
        assert_eq!(opreturn_script, Script::from_str("6a303545577453636e65347a57734761503467566f38446d4c7043685678334d7a6f5154704b4a4345644254594441314479").unwrap());
        if let Some(info) = extract_opreturn_data(&opreturn_script)
            .and_then(|opreturn| OpReturnExtractor::extract_account(&opreturn))
        {
            account_info = Some(info);
            assert_eq!(account_info, op_return);
            break;
        }
    }
    let script = Script::from_str("6a303545577453636e65347a57734761503467566f38446d4c7043685678334d7a6f5154704b4a4345644254594441314479").unwrap();
    let data = extract_opreturn_data(&script).unwrap();
    let account = from_ss58_check(&data).unwrap();
    assert_eq!(
        account,
        "5EWtScne4zWsGaP4gVo8DmLpChVx3MzoQTpKJCEdBTYDA1Dy"
            .parse::<AccountId32>()
            .unwrap(),
    );
}
