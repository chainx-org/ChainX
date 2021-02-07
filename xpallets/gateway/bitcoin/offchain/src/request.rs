// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

use frame_support::debug;
use light_bitcoin::{
    chain::{Block as BtcBlock, Transaction as BtcTransaction},
    keys::Network as BtcNetwork,
    primitives::{hash_rev, H256 as BtcHash},
    serialization::{deserialize, Reader},
};
use sp_runtime::offchain::{http, http::PendingRequest, Duration};
use sp_std::{str, vec, vec::Vec};

use crate::{Error, Module, Trait};

pub const RETRY_NUM: u32 = 5;
pub const MAX_RETRY_NUM: u32 = 5;
const SEND_RAW_TX_ERR_PREFIX: &str = "send raw transaction RPC error: ";

struct SendRawTxError {
    code: i64,
    message: String,
}

impl<T: Trait> Module<T> {
    /// Http post request
    pub(crate) fn post<B, I>(url: &str, req_body: B) -> Result<Vec<u8>, Error<T>>
    where
        B: Default + IntoIterator<Item = I>,
        I: AsRef<[u8]>,
    {
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
        let pending = http::Request::post(url, req_body)
            .deadline(deadline)
            .send()
            .map_err(Error::<T>::from)?;
        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::HttpDeadlineReached)??;
        let resp_body = response.body().collect::<Vec<u8>>();
        Ok(resp_body)
    }

    /// Http get request
    pub(crate) fn get<U: AsRef<str>>(url: U) -> Result<Vec<u8>, Error<T>> {
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
        let pending = http::Request::get(url.as_ref())
            .deadline(deadline)
            .send()
            .map_err(Error::<T>::from)?;
        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::HttpDeadlineReached)??;
        let resp_body = response.body().collect::<Vec<u8>>();
        Ok(resp_body)
    }

    /// Get transaction's PendingRequest
    pub(crate) fn get_transactions_pending(
        hash: &str,
        network: BtcNetwork,
    ) -> Result<http::PendingRequest, Error<T>> {
        let url = match network {
            BtcNetwork::Mainnet => format!("https://blockstream.info/api/tx/{}/raw", hash),
            BtcNetwork::Testnet => format!("https://blockstream.info/testnet/api/tx/{}/raw", hash),
        };
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
        let pending = http::Request::get(url.as_ref())
            .deadline(deadline)
            .send()
            .map_err(Error::<T>::from)?;
        Ok(pending)
    }

    /// Get all responses and return all transactions
    pub(crate) fn get_all_transactions(
        pending: Vec<PendingRequest>,
    ) -> Result<Vec<BtcTransaction>, Error<T>> {
        let responses = PendingRequest::wait_all(pending);
        let mut transactions = Vec::<BtcTransaction>::new();
        for response in responses {
            let body = response?.body().collect::<Vec<u8>>();
            let transaction = deserialize::<_, BtcTransaction>(Reader::new(&body))
                .map_err(|_| Error::<T>::BtcSserializationError)?;
            transactions.push(transaction);
        }
        Ok(transactions)
    }

    /// Get btc block hash from btc network
    pub(crate) fn fetch_block_hash(
        height: u32,
        network: BtcNetwork,
    ) -> Result<Option<String>, Error<T>> {
        let url = match network {
            BtcNetwork::Mainnet => format!("https://blockstream.info/api/block-height/{}", height),
            BtcNetwork::Testnet => format!(
                "https://blockstream.info/testnet/api/block-height/{}",
                height
            ),
        };
        let resp_body = Self::get(url)?;
        let resp_body = str::from_utf8(&resp_body).map_err(|_| {
            debug::warn!("No UTF8 body");
            Error::<T>::HttpBodyNotUTF8
        })?;
        const RESP_BLOCK_NOT_FOUND: &str = "Block not found";
        if resp_body == RESP_BLOCK_NOT_FOUND {
            Ok(None)
        } else {
            let hash: String = resp_body.into();
            Ok(Some(hash))
        }
    }

    /// Get btc block from btc network
    pub(crate) fn fetch_block(hash: &str, network: BtcNetwork) -> Result<BtcBlock, Error<T>> {
        let url = match network {
            BtcNetwork::Mainnet => format!("https://blockstream.info/api/block/{}/raw", hash),
            BtcNetwork::Testnet => {
                format!("https://blockstream.info/testnet/api/block/{}/raw", hash)
            }
        };
        let body = Self::get(url)?;
        let block = deserialize::<_, BtcBlock>(Reader::new(&body))
            .map_err(|_| Error::<T>::BtcSserializationError)?;
        Ok(block)
    }

    /// Get transaction from btc network
    #[allow(dead_code)]
    pub(crate) fn fetch_transaction(
        hash: &str,
        network: BtcNetwork,
    ) -> Result<BtcTransaction, Error<T>> {
        let url = match network {
            BtcNetwork::Mainnet => format!("https://blockstream.info/api/tx/{}/raw", hash),
            BtcNetwork::Testnet => format!("https://blockstream.info/testnet/api/tx/{}/raw", hash),
        };
        let body = Self::get(url)?;
        let transaction = deserialize::<_, BtcTransaction>(Reader::new(&body))
            .map_err(|_| Error::<T>::BtcSserializationError)?;
        debug::info!("₿ Transaction {}", hash_rev(transaction.hash()));
        Ok(transaction)
    }

    /// Broadcast raw transaction to btc network
    pub(crate) fn send_raw_transaction<TX: AsRef<[u8]>>(
        hex_tx: TX,
        network: BtcNetwork,
    ) -> Result<String, Error<T>> {
        let url = match network {
            BtcNetwork::Mainnet => "https://blockstream.info/api/tx",
            BtcNetwork::Testnet => "https://blockstream.info/testnet/api/tx",
        };
        let resp_body = Self::post(url, vec![hex_tx.as_ref()])?;
        let resp_body = str::from_utf8(&resp_body).map_err(|_| {
            debug::warn!("No UTF8 body");
            Error::<T>::HttpBodyNotUTF8
        })?;

        if resp_body.len() == 2 * BtcHash::len_bytes() {
            let hash: String = resp_body.into();
            debug::info!(
                "₿ Send Transaction successfully, Hash: {}, HexTx: {}",
                hash,
                hex::encode(hex_tx.as_ref())
            );
            Ok(hash)
        } else if resp_body.starts_with(SEND_RAW_TX_ERR_PREFIX) {
            if let Some(err) = Self::parse_send_raw_tx_error(resp_body) {
                debug::info!(
                    "₿ Send Transaction error: (code: {}, msg: {}), HexTx: {}",
                    err.code,
                    err.message,
                    hex::encode(hex_tx.as_ref())
                );
            } else {
                debug::info!(
                    "₿ Send Transaction unknown error, HexTx: {}",
                    hex::encode(hex_tx.as_ref())
                );
            }
            Err(Error::<T>::BtcSendRawTxError)
        } else {
            debug::info!(
                "₿ Send Transaction unknown error, HexTx: {}",
                hex::encode(hex_tx.as_ref())
            );
            Err(Error::<T>::BtcSendRawTxError)
        }
    }

    /// Parse broadcast's error
    fn parse_send_raw_tx_error(resp_body: &str) -> Option<SendRawTxError> {
        use lite_json::JsonValue;
        let rest_resp = resp_body.trim_start_matches(SEND_RAW_TX_ERR_PREFIX);
        let value = lite_json::parse_json(rest_resp).ok();
        value.and_then(|v| match v {
            JsonValue::Object(obj) => {
                let code = obj
                    .iter()
                    .find(|(k, _)| k == &['c', 'o', 'd', 'e'])
                    .map(|(_, code)| code);
                let message = obj
                    .iter()
                    .find(|(k, _)| k == &['m', 'e', 's', 's', 'a', 'g', 'e'])
                    .map(|(_, msg)| msg);
                match (code, message) {
                    (Some(JsonValue::Number(code)), Some(JsonValue::String(msg))) => {
                        Some(SendRawTxError {
                            code: code.integer,
                            message: msg.iter().collect(),
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use light_bitcoin::{
        keys::Network as BtcNetwork,
        primitives::{h256, hash_rev},
    };
    use sp_core::offchain::{testing, OffchainExt};
    use sp_io::TestExternalities;

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
        uri: "https://blockstream.info/api/tx/4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b/raw".into(),
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

    #[test]
    fn parse_send_raw_tx_err() {
        let resp_body = r#"send raw transaction RPC error: {"code":-25,"message":"bad-txns-inputs-missingorspent"}"#;
        let err = XGatewayBitcoinRelay::parse_send_raw_tx_error(resp_body).unwrap();
        assert_eq!(err.code, -25);
        assert_eq!(err.message, "bad-txns-inputs-missingorspent");
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
}
