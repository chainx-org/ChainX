// Copyright 2019 Chainpool
pub mod handler;
pub mod utils;
mod validator;

use crate::rstd::prelude::*;
use crate::rstd::result::Result as StdResult;

use crate::btc_chain::Transaction;
use crate::btc_keys::Address;
use crate::btc_primitives::hash::H256;
use crate::btc_script::{builder, script::Script, Opcode};
use crate::crypto::dhash160;

use crate::runtime_primitives::traits::As;
use crate::support::{dispatch::Result, StorageMap};

use crate::types::{RelayTx, TxType};
use crate::{xaccounts, xrecords, InputAddrFor, Module, RawEvent, Trait, TxFor};

use self::handler::TxHandler;
use self::utils::{
    get_trustee_address, inspect_address_from_transaction, is_key, parse_addr_from_script,
};
pub use self::validator::{parse_and_check_signed_tx, validate_transaction};

#[cfg(feature = "std")]
use crate::hash_strip;
use crate::xsupport::{debug, error};

pub fn detect_transaction_type<T: Trait>(tx: &RelayTx) -> StdResult<TxType, &'static str> {
    // detect withdraw
    // only use first input
    let outpoint = &tx.raw.inputs[0].previous_output;
    let send_address = match inspect_address_from_transaction::<T>(&tx.previous_raw, outpoint) {
        Some(a) => a,
        None => return Err("Inspect address failed in this transaction"),
    };

    let trustee_addr = get_trustee_address::<T>()?;

    if send_address.hash == trustee_addr.hash {
        return Ok(TxType::Withdraw);
    }

    // detect deposit
    for output in tx.raw.outputs.iter() {
        if is_key::<T>(&output.script_pubkey.to_vec().into(), &trustee_addr) {
            return Ok(TxType::Deposit);
        }
    }
    error!("[detect_transaction_type]|not find trustee_addr in tx outputs|outputs:{:?}|trustee_addr:{:?}", tx.raw.outputs, trustee_addr);
    Err("Irrelevant tx")
}

pub fn handle_tx<T: Trait>(txid: &H256) -> Result {
    let tx_handler = TxHandler::new::<T>(txid)?;
    tx_handler.handle::<T>()?;
    // if success, remove handled tx
    remove_unused_tx::<T>(txid);
    Ok(())
}

pub fn remove_unused_tx<T: Trait>(txid: &H256) {
    debug!(
        "[remove_unused_tx]|remove old tx|tx_hash:{:}...",
        hash_strip(txid)
    );
    TxFor::<T>::remove(txid);
    InputAddrFor::<T>::remove(txid);
}

pub fn create_multi_address<T: Trait>(pubkeys: Vec<Vec<u8>>) -> Option<(Address, Script)> {
    let (sig_num, trustee_num) = get_sig_num::<T>();
    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + sig_num as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let mut build = builder::Builder::default().push_opcode(opcode);
    for (_, pubkey) in pubkeys.iter().enumerate() {
        build = build.push_bytes(pubkey);
    }

    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + trustee_num as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let redeem_script = build
        .push_opcode(opcode)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();
    let network_id = Module::<T>::network_id(); // <NetworkId<T>>::get();
    let net = if network_id == 1 {
        keys::Network::Testnet
    } else {
        keys::Network::Mainnet
    };
    let addr = Address {
        kind: keys::Type::P2SH,
        network: net,
        hash: dhash160(&redeem_script),
    };
    Some((addr, redeem_script))
}

/// Check that the cash withdrawal transaction is correct
pub fn check_withdraw_tx<T: Trait>(tx: &Transaction, withdrawal_id_list: &[u32]) -> Result {
    match Module::<T>::withdrawal_proposal() {
        Some(_) => Err("Unfinished withdrawal transaction"),
        None => {
            let trustee_address: Address = get_trustee_address::<T>()?;
            // withdrawal addr list for account withdrawal application
            let mut appl_withdrawal_list = Vec::new();
            for withdraw_index in withdrawal_id_list.iter() {
                if let Some(record) = xrecords::Module::<T>::application_map(withdraw_index) {
                    // record.data.addr() is base58
                    // verify btc address would convert a base58 addr to Address
                    let addr: Address = Module::<T>::verify_btc_address(&record.data.addr())
                        .map_err(|_| "Parse addr error")?;
                    appl_withdrawal_list.push((addr, record.data.balance().as_() as u64));
                } else {
                    return Err("Withdraw id not in withdrawal ApplicationMap record");
                }
            }
            // withdrawal addr list for tx outputs
            let btc_withdrawal_fee = Module::<T>::btc_withdrawal_fee();
            let mut tx_withdraw_list = Vec::new();
            for output in tx.outputs.iter() {
                let script: Script = output.script_pubkey.clone().into();
                let addr =
                    parse_addr_from_script::<T>(&script).ok_or("not found addr in this out")?;

                if addr.hash != trustee_address.hash {
                    // expect change to trustee_addr output
                    tx_withdraw_list.push((addr, output.value + btc_withdrawal_fee));
                }
            }

            // appl_withdrawal_list must match to tx_withdraw_list
            if appl_withdrawal_list.len() != tx_withdraw_list.len() {
                error!("withdrawal tx's outputs not equal to withdrawal application list, withdrawal application len:{:}, withdrawal tx's outputs len:{:}|withdrawal application list:{:?}, tx withdrawal outputs:{:?}",
                       appl_withdrawal_list.len(), tx_withdraw_list.len(),
                       withdrawal_id_list.iter().zip(appl_withdrawal_list).collect::<Vec<_>>(),
                       tx_withdraw_list
                );
                return Err("withdrawal tx's outputs not equal to withdrawal application list");
            }

            for item in appl_withdrawal_list.iter() {
                if !tx_withdraw_list.contains(item) {
                    error!(
                        "withdrawal tx's output not match to withdrawal application. withdrawal application list:{:?}, tx withdrawal outputs:{:?}",
                        withdrawal_id_list.iter().zip(appl_withdrawal_list).collect::<Vec<_>>(),
                        tx_withdraw_list
                    );
                    return Err("withdrawal tx's output not match to withdrawal application");
                }
            }
            Ok(())
        }
    }
}

/// Get the required number of signatures
/// sig_num: Number of signatures required
/// trustee_num: Total number of multiple signatures
/// NOTE: Signature ratio greater than 2/3
pub fn get_sig_num<T: Trait>() -> (usize, usize) {
    let trustee_list = xaccounts::Module::<T>::trustee_intentions();
    let trustee_num = trustee_list.len();
    let sig_num = match 2_usize.checked_mul(trustee_num) {
        Some(m) => {
            if m % 3 == 0 {
                m / 3
            } else {
                m / 3 + 1
            }
        }
        None => 0,
    };
    (sig_num, trustee_num)
}

/// Update the signature status of trustee
/// state: false -> Veto signature, true -> Consent signature
pub fn update_trustee_vote_state<T: Trait>(
    state: bool,
    who: &T::AccountId,
    trustee_list: &mut Vec<(T::AccountId, bool)>,
) {
    match trustee_list.iter_mut().find(|ref info| info.0 == *who) {
        Some((_, old_state)) => {
            // if account is exist, override state
            *old_state = state;
        }
        None => {
            trustee_list.push((who.clone(), state));
        }
    }
    Module::<T>::deposit_event(RawEvent::UpdateSignWithdrawTx(who.clone(), state));
}
