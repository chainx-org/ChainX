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
