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
use sp_core::crypto::{set_default_ss58_version, AccountId32, Ss58AddressFormat};
use xp_gateway_bitcoin::{AccountExtractor, BtcTxTypeDetector, OpReturnExtractor};

#[test]
fn fetch_block_hash() {
    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = TestExternalities::default();
    t.register_extension(OffchainExt::new(offchain));

    state.write().expect_request(testing::PendingRequest {
        method: "GET".into(),
        uri: "https://blockstream.info/api/block-height/0".into(),
        response: Some(
            "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
                .as_bytes()
                .to_vec(),
        ),
        sent: true,
        ..Default::default()
    });

    t.execute_with(|| {
        let hash = XGatewayBitcoinRelay::fetch_block_hash(0, BtcNetwork::Mainnet).unwrap();
        assert_eq!(
            hash.unwrap(),
            "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
        );
    });
}

#[test]
fn fetch_block() {
    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = TestExternalities::default();
    t.register_extension(OffchainExt::new(offchain));

    state.write().expect_request(testing::PendingRequest {
        method: "GET".into(),
        uri: "https://blockstream.info/api/block/000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f/raw".into(),
        response: Some(hex::decode("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c0101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000").unwrap()),
        sent: true,
        ..Default::default()
    });

    t.execute_with(|| {
        let block = XGatewayBitcoinRelay::fetch_block(
            "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
            BtcNetwork::Mainnet,
        )
        .unwrap();
        assert_eq!(
            hash_rev(block.hash()),
            h256("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")
        );
    });
}

#[test]
fn fetch_transaction() {
    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = TestExternalities::default();
    t.register_extension(OffchainExt::new(offchain));

    state.write().expect_request(testing::PendingRequest {
        method: "GET".into(),
        uri: "https://blockstream.info/api/tx/4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b/hex".into(),
        response: Some(hex::decode("01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000").unwrap()),
        sent: true,
        ..Default::default()
    });

    t.execute_with(|| {
        let tx = XGatewayBitcoinRelay::fetch_transaction(
            "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            BtcNetwork::Mainnet,
        )
        .unwrap();
        assert_eq!(
            hash_rev(tx.hash()),
            h256("4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b")
        );
    });
}

#[ignore]
#[test]
fn send_raw_transaction() {
    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = TestExternalities::default();
    t.register_extension(OffchainExt::new(offchain));

    state.write().expect_request(testing::PendingRequest {
        method: "POST".into(),
        uri: "https://blockstream.info/api/tx".into(),
        response: Some(r#"sendrawtransaction RPC error: {"code":-25,"message":"bad-txns-inputs-missingorspent"}"#.as_bytes().to_vec()),
        sent: true,
        ..Default::default()
    });

    t.execute_with(|| {
        let rawtx = hex::decode("01000000011935b41d12936df99d322ac8972b74ecff7b79408bbccaf1b2eb8015228beac8000000006b483045022100921fc36b911094280f07d8504a80fbab9b823a25f102e2bc69b14bcd369dfc7902200d07067d47f040e724b556e5bc3061af132d5a47bd96e901429d53c41e0f8cca012102152e2bb5b273561ece7bbe8b1df51a4c44f5ab0bc940c105045e2cc77e618044ffffffff0240420f00000000001976a9145fb1af31edd2aa5a2bbaa24f6043d6ec31f7e63288ac20da3c00000000001976a914efec6de6c253e657a9d5506a78ee48d89762fb3188ac00000000").unwrap();
        assert!(XGatewayBitcoinRelay::send_raw_transaction(rawtx, BtcNetwork::Mainnet).is_err());
    });
}

#[test]
fn parse_send_raw_tx_err() {
    let resp_body =
        r#"sendrawtransaction RPC error: {"code":-25,"message":"bad-txns-inputs-missingorspent"}"#;
    let err = XGatewayBitcoinRelay::parse_send_raw_tx_error(resp_body).unwrap();
    assert_eq!(err.code, -25);
    assert_eq!(err.message, "bad-txns-inputs-missingorspent");
}

#[test]
fn detect_transaction_type() {
    set_default_ss58_version(Ss58AddressFormat::ChainXAccount);

    const DEPOSIT_HOT_ADDR: &str = "3LFSUKkP26hun42J1Dy6RATsbgmBJb27NF";
    const DEPOSIT_COLD_ADDR: &str = "3FLBhPfEqmw4Wn5EQMeUzPLrQtJMprgwnw";
    let btc_tx_detector = BtcTxTypeDetector::new(
        Network::Mainnet,
        0,
        (
            DEPOSIT_HOT_ADDR.parse::<Address>().unwrap(),
            DEPOSIT_COLD_ADDR.parse::<Address>().unwrap(),
        ),
        None,
    );
    let case = (
                "020000000001012f0f1be54334c36baf9edce4051acfcc4634e27504e39bc6466a1dadd36110e40100000017160014cd286c8c974540b1019e351c33551dc152e7447bffffffff03307500000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587672400000000000017a9149b995c9fddc8e5086626f7123631891a209d83a4870000000000000000326a3035556a336568616d445a57506667413869415a656e6863416d5044616b6a6634614d626b424234645856766a6f57367802483045022100f27347145406cc9706cd4d83018b07303c30b8d43f935019bf1d3accb38696f70220546db7a30dc8f0c4f02e17460573d009d26d85bd98a32642e88c6f74e76ac7140121037788522b753d5517cd9191c96f741a0d2b479369697d41567b4b418c7979d77300000000".parse::<Transaction>().unwrap(),
                (
                    Some((
                        "5Uj3ehamDZWPfgA8iAZenhcAmPDakjf4aMbkBB4dXVvjoW6x".parse::<AccountId32>().unwrap(),
                        None
                    )),
                    30000
                )
            );
    let (tx, result) = case;
    let (result, value) = result;
    let (op_return, deposit_value) =
        btc_tx_detector.parse_deposit_transaction_outputs(&tx, OpReturnExtractor::extract_account);
    assert_eq!(op_return, result);
    assert_eq!(deposit_value, value);
}
