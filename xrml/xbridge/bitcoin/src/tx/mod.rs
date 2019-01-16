// Copyright 2018 Chainpool

use codec::Decode;
use rstd::marker::PhantomData;
use rstd::prelude::*;
use rstd::result::Result as StdResult;

use super::{
    AccountMap, AddressMap, BlockHeaderFor, CandidateTx, CertAddress, Module, NetworkId,
    PendingDepositMap, Trait, TrusteeAddress, TxFor, TxInfo, TxProposal, TxType, UTXOKey, UTXOSet,
    UTXOSetKey, UTXOStatus, UTXO,
};

use chain::{OutPoint, Transaction, TransactionInput, TransactionOutput};
use keys;
use merkle::{parse_partial_merkle_tree, PartialMerkleTree};
use script::{
    builder, script::Script, SignatureChecker, SignatureVersion, TransactionInputSigner,
    TransactionSignatureChecker,
};

use primitives::{bytes::Bytes, hash::H256};
use runtime_io;
use runtime_primitives::traits::As;
use runtime_support::{dispatch::Result, StorageMap, StorageValue};

use xaccounts;
use xrecords;

pub use self::proposal::{handle_proposal, Proposal};
pub use self::validator::validate_transaction;

mod extracter;
mod handler;
mod proposal;
mod validator;

use self::handler::TxHandler;

#[derive(Clone, Encode, Decode)]
pub struct RelayTx {
    pub block_hash: H256,
    pub raw: Transaction,
    pub merkle_proof: PartialMerkleTree,
    pub previous_raw: Transaction,
}

const OP_RETURN_FLAG: &'static [u8] = b"ChainX";

fn is_key(script_pubkey: &[u8], trustee_address: &keys::Address) -> bool {
    let script: Vec<u8> = script_pubkey.iter().cloned().collect();
    let script: Script = script.into();
    let script_addresses = script.extract_destinations().unwrap_or(Vec::new());
    if script_addresses.len() == 1 && trustee_address.hash == script_addresses[0].hash {
        return true;
    }
    false
}

pub fn select_utxo<T: Trait>(balance: u64) -> Option<Vec<UTXO>> {
    if let Some(keys) = <UTXOSetKey<T>>::get() {
        let mut count_balance = 0;
        let mut utxo_set = Vec::new();

        let is_avaliable = |utxo: &UTXOStatus| -> bool { utxo.status && utxo.balance > 0 };

        for key in keys {
            let utxo = <UTXOSet<T>>::get(&key);

            if is_avaliable(&utxo) {
                count_balance += utxo.balance;
                utxo_set.push(UTXO {
                    txid: key.txid,
                    index: key.index,
                    balance: utxo.balance,
                });
                if count_balance >= balance {
                    return Some(utxo_set);
                }
            }
        }
    }

    None
}

pub fn inspect_address<T: Trait>(tx: &Transaction, outpoint: OutPoint) -> Option<keys::Address> {
    let script: Script = tx.outputs[outpoint.index as usize]
        .script_pubkey
        .clone()
        .into();
    let script_addresses = script.extract_destinations().unwrap_or(Vec::new());
    if script_addresses.len() == 1 {
        let address = &script_addresses[0];
        let network_id = <NetworkId<T>>::get();
        let network = if network_id == 1 {
            keys::Network::Testnet
        } else {
            keys::Network::Mainnet
        };
        let address = keys::Address {
            kind: address.kind,
            network: network,
            hash: address.hash.clone(),
        };
        return Some(address);
    }
    return None;
}

pub fn handle_tx<T: Trait>(txid: &H256) -> Result {
    let trustee_address = <TrusteeAddress<T>>::get().ok_or("Should set RECEIVE_address first.")?;
    let cert_address = <CertAddress<T>>::get().ok_or("Should set CERT_address first.")?;
    let tx_info = <TxFor<T>>::get(txid);
    let input_address = tx_info.input_address;

    let tx_handler = TxHandler::new(&txid);

    match get_tx_type(&input_address, &trustee_address, &cert_address) {
        TxType::Withdraw => {
            tx_handler.withdraw::<T>(&trustee_address)?;
        }
        TxType::SendCert => {
            tx_handler.cert::<T>()?;
        }
        TxType::Deposit => {
            for output in tx_info.raw_tx.outputs.iter() {
                if is_key(&output.script_pubkey, &trustee_address) {
                    tx_handler.deposit::<T>(&trustee_address);
                    break;
                }
            }
        }
        _ => return Err("Unknow tx type"),
    };

    Ok(())
}

fn get_tx_type(
    input_address: &keys::Address,
    trustee_address: &keys::Address,
    cert_address: &keys::Address,
) -> TxType {
    if input_address.hash == trustee_address.hash {
        return TxType::Withdraw;
    } else if input_address.hash == cert_address.hash {
        return TxType::SendCert;
    } else {
        return TxType::Deposit;
    }
}
