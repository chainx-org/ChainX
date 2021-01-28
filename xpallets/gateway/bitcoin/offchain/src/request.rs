#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

use light_bitcoin::{
    chain::{Block as BtcBlock, BlockHeader as BtcHeader, Transaction as BtcTransaction},
    keys::{Address as BtcAddress, Network as BtcNetwork},
    merkle::PartialMerkleTree,
    primitives::{hash_rev, H256 as BtcHash},
    serialization::{deserialize, serialize, Reader},
};

use sp_runtime::offchain::{
    http,
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
    Duration,
};
use sp_std::{
    collections::btree_set::BTreeSet, convert::TryFrom, marker::PhantomData, str, vec, vec::Vec,
};

use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchResultWithPostInfo, Parameter},
    traits::Get,
    weights::Pays,
    StorageValue,
};

use crate::{Error, Module, Trait};

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
        // Set timeout
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
        // Http post request
        let pending = http::Request::post(url, req_body)
            .deadline(deadline)
            .send()
            .map_err(Error::<T>::from)?;
        // Http response
        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::HttpDeadlineReached)??;
        // Let's check the status code before we proceed to reading the response.
        if response.code != 200 {
            debug::warn!("Unexpected status code: {}", response.code);
            return Err(Error::<T>::HttpUnknown);
        }
        // Response body
        let resp_body = response.body().collect::<Vec<u8>>();
        Ok(resp_body)
    }

    /// Http get request
    pub(crate) fn get<U: AsRef<str>>(url: U) -> Result<Vec<u8>, Error<T>> {
        // Set timeout
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
        // Http get request
        let pending = http::Request::get(url.as_ref())
            .deadline(deadline)
            .send()
            .map_err(Error::<T>::from)?;
        // Http response
        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::HttpDeadlineReached)??;
        // Let's check the status code before we proceed to reading the response.
        if response.code != 200 {
            debug::warn!("Unexpected status code: {}", response.code);
            return Err(Error::<T>::HttpUnknown);
        }
        // Response body
        let resp_body = response.body().collect::<Vec<u8>>();
        Ok(resp_body)
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
            debug::info!("₿ Block #{} not found", height);
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
