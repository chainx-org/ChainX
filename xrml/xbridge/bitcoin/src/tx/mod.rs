// Copyright 2019 Chainpool

use rstd::prelude::*;
use rstd::result::Result as StdResult;

use super::{
    BindStatus, BlockHeaderFor, BtcFee, CandidateTx, DepositCache, Module, NetworkId,
    PendingDepositMap, RawEvent, Trait, TrusteeRedeemScript, TxFor, TxInfo, TxProposal, TxType,
    VoteResult,
};
use chain::{OutPoint, Transaction};
use crypto::dhash160;
use keys;
use keys::{Address, DisplayLayout};
use merkle::{parse_partial_merkle_tree, PartialMerkleTree};
use primitives::{bytes::Bytes, hash::H256};
use runtime_io;
use runtime_primitives::traits::As;
use runtime_support::{dispatch::Result, StorageMap, StorageValue};
use script::{
    builder, script::Script, Opcode, SignatureChecker, SignatureVersion, TransactionInputSigner,
    TransactionSignatureChecker,
};
use xrecords;
use xrecords::ApplicationMap;

pub use self::validator::{handle_condidate, validate_transaction};

mod handler;
mod validator;
use self::handler::TxHandler;
use xr_primitives::generic::b58;

#[derive(Clone, Encode, Decode)]
pub struct RelayTx {
    pub block_hash: H256,
    pub raw: Transaction,
    pub merkle_proof: PartialMerkleTree,
    pub previous_raw: Transaction,
}

fn is_key(script_pubkey: &[u8], trustee_address: &Address) -> bool {
    let script: Vec<u8> = script_pubkey.iter().cloned().collect();
    let script: Script = script.into();
    let script_addresses = script.extract_destinations().unwrap_or_default();
    if script_addresses.len() == 1 && trustee_address.hash == script_addresses[0].hash {
        return true;
    }
    false
}

fn get_tx_type(input_address: &Address, trustee_address: &Address) -> TxType {
    if input_address.hash == trustee_address.hash {
        TxType::Withdraw
    } else {
        TxType::Deposit
    }
}

pub fn inspect_address<T: Trait>(tx: &Transaction, outpoint: OutPoint) -> Option<Address> {
    let script: Script = tx.outputs[outpoint.index as usize]
        .script_pubkey
        .clone()
        .into();
    let script_addresses = script.extract_destinations().unwrap_or_default();
    if script_addresses.len() == 1 {
        let address = &script_addresses[0];
        let network_id = <NetworkId<T>>::get();
        let net = if network_id == 1 {
            keys::Network::Testnet
        } else {
            keys::Network::Mainnet
        };
        let address = Address {
            kind: address.kind,
            network: net,
            hash: address.hash.clone(),
        };
        return Some(address);
    }
    None
}

pub fn handle_tx<T: Trait>(txid: &H256) -> Result {
    let trustee_address = <xaccounts::TrusteeAddress<T>>::get(xassets::Chain::Bitcoin)
        .ok_or("Should set trustee address first.")?;
    let hot_address = Address::from_layout(&trustee_address.hot_address.as_slice())
        .map_err(|_| "Invalid address")?;
    let tx_info = <TxFor<T>>::get(txid);
    let input_address = tx_info.input_address;

    let tx_handler = TxHandler::new(&txid);

    match get_tx_type(&input_address, &hot_address) {
        TxType::Withdraw => {
            tx_handler.withdraw::<T>().map_err(|e| {
                <TxFor<T>>::remove(txid);
                e
            })?;
        }
        TxType::Deposit => {
            for output in tx_info.raw_tx.outputs.iter() {
                if is_key(&output.script_pubkey, &hot_address) {
                    tx_handler.deposit::<T>(&hot_address);
                    break;
                }
            }
        }
        _ => {
            <TxFor<T>>::remove(txid);
            return Err("Unknow tx type");
        }
    };
    <TxFor<T>>::remove(txid);
    Ok(())
}

pub fn create_multi_address<T: Trait>(pubkeys: Vec<Vec<u8>>) -> Option<(Address, Script)> {
    let (sign_num, node_num) = sign_num::<T>();
    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + sign_num as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let mut build = builder::Builder::default().push_opcode(opcode);
    for (_, pubkey) in pubkeys.iter().enumerate() {
        build = build.push_bytes(pubkey);
    }

    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + node_num as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let script = build
        .push_opcode(opcode)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();
    let addr = Address {
        kind: keys::Type::P2SH,
        network: keys::Network::Testnet,
        hash: dhash160(&script),
    };
    Some((addr, script))
}

pub fn check_withdraw_tx<T: Trait>(
    tx: Transaction,
    withdraw_id: Vec<u32>,
    trustee_address: Address,
) -> Result {
    match <TxProposal<T>>::get() {
        Some(_) => Err("Unfinished withdrawal transaction"),
        None => {
            let mut addr_flag = false;
            let mut multi_flag = false;
            let withdraw_len = withdraw_id.len();
            let output_len = tx.outputs.len();
            let btc_fee = <BtcFee<T>>::get();
            for withdraw_index in withdraw_id.clone().into_iter() {
                match ApplicationMap::<T>::get(&withdraw_index) {
                    Some(r) => {
                        let addr: Address = Module::<T>::verify_btc_address(&r.data.addr())
                            .map_err(|_| "Parse addr error")?;

                        for (_, output) in tx.outputs.iter().enumerate() {
                            let script = &output.script_pubkey;
                            let into_script: Script = script.clone().into();

                            let script_addresses =
                                into_script.extract_destinations().unwrap_or_default();
                            if script_addresses.len() == 1 {
                                if addr.hash == script_addresses[0].hash
                                    && output.value + btc_fee == r.data.balance().as_() as u64
                                {
                                    addr_flag = true;
                                    break;
                                } else if trustee_address.hash == script_addresses[0].hash {
                                    multi_flag = true;
                                }
                            }
                        }
                        if !addr_flag {
                            return Err("The withdraw tx info not match the withdraw list");
                        }
                        addr_flag = false;
                    }
                    None => {
                        return Err("Withdraw id not in withdraw ApplicationMap record");
                    }
                }
            }
            if output_len == withdraw_len + 1 && !multi_flag {
                return Err("The change address not match the trustee address");
            }
            let candidate = CandidateTx::new(withdraw_id, tx, VoteResult::Unfinish, Vec::new());
            <TxProposal<T>>::put(candidate);
            runtime_io::print("[bridge-btc] Through the legality check of withdrawal transaction ");
            Ok(())
        }
    }
}

pub fn sign_num<T: Trait>() -> (usize, usize) {
    let node_list = <xaccounts::TrusteeIntentions<T>>::get();
    let node_num = node_list.len();
    let sign_num = match 2_usize.checked_mul(node_num) {
        Some(m) => {
            if m % 3 == 0 {
                m / 3
            } else {
                m / 3 + 1
            }
        }
        None => 0,
    };
    (sign_num, node_num)
}
