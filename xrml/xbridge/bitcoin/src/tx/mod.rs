// Copyright 2018-2019 Chainpool.

pub mod handler;
pub mod utils;
mod validator;

// Substrate
use primitives::traits::As;
use rstd::{prelude::*, result};
use support::{dispatch::Result, StorageMap};

// ChainX
use xassets::Chain;
use xsupport::{debug, error};

// light-bitcoin
use btc_chain::Transaction;
use btc_crypto::dhash160;
use btc_keys::{Address, Type};
use btc_primitives::{Bytes, H256};
use btc_script::{Builder, Opcode, Script};

use crate::types::{RelayTx, TrusteeAddrInfo, TxType};
use crate::{InputAddrFor, Module, RawEvent, Trait, TxFor};

use self::handler::TxHandler;
use self::utils::{
    equal_addr, get_hot_trustee_address, get_networkid, get_trustee_address_pair,
    inspect_address_from_transaction, parse_addr_from_script,
};
pub use self::validator::{parse_and_check_signed_tx, validate_transaction};

/// parse tx's inputs/outputs into Option<Address>
/// e.g
/// notice the relay tx only has first input
///        _________
///  addr |        | Some(addr)
///       |   tx   | Some(addr)
///       |________| None (OP_RETURN or something unknown)
/// then judge type
pub fn detect_transaction_type<T: Trait>(
    relay_tx: &RelayTx,
) -> result::Result<TxType, &'static str> {
    let current_session_number = xaccounts::Module::<T>::current_session_number(Chain::Bitcoin);
    let (hot_addr, cold_addr) = get_trustee_address_pair::<T>(current_session_number)?;
    // parse input addr
    let outpoint = &relay_tx.raw.inputs[0].previous_output;
    let input_addr = match inspect_address_from_transaction::<T>(&relay_tx.previous_raw, outpoint) {
        Some(a) => a,
        None => return Err("Inspect address failed in this transaction"),
    };
    // parse output addr
    let outputs: Vec<Option<Address>> = relay_tx
        .raw
        .outputs
        .iter()
        .map(|out| parse_addr_from_script::<T>(&out.script_pubkey.to_vec().into()))
        .collect();
    // ---------- parse finish
    // judge input has trustee addr
    let input_is_trustee =
        equal_addr(&input_addr, &hot_addr) || equal_addr(&input_addr, &cold_addr);
    // judge if all outputs contains hot/cold trustee
    let all_outputs_trustee = outputs.iter().all(|item| {
        if let Some(addr) = item {
            if equal_addr(addr, &hot_addr) || equal_addr(addr, &cold_addr) {
                return true;
            }
        }
        false
    });
    // judge tx type
    if input_is_trustee {
        if all_outputs_trustee {
            return Ok(TxType::HotAndCold);
        }
        // outputs contains other addr, it's user addr, thus it's a withdrawal
        return Ok(TxType::Withdrawal);
    } else {
        let last_number = Module::<T>::last_trustee_session_number();
        if let Ok((old_hot_addr, old_cold_addr)) = get_trustee_address_pair::<T>(last_number) {
            let input_is_old_trustee =
                equal_addr(&input_addr, &old_hot_addr) || equal_addr(&input_addr, &old_cold_addr);
            if input_is_old_trustee && all_outputs_trustee {
                // input should from old trustee addr, outputs should all be current trustee addrs
                return Ok(TxType::TrusteeTransition);
            }
        }
        // any output contains hot trustee addr
        let check_outputs = outputs.iter().any(|item| {
            if let Some(addr) = item {
                // only hot addr for deposit
                if equal_addr(addr, &hot_addr) {
                    return true;
                }
            }
            false
        });
        if check_outputs {
            return Ok(TxType::Deposit);
        }
    }

    Ok(TxType::Irrelevance)
}

pub fn handle_tx<T: Trait>(txid: &H256) -> Result {
    let tx_handler = TxHandler::new::<T>(txid)?;
    tx_handler.handle::<T>()?;
    Ok(())
}

pub fn remove_unused_tx<T: Trait>(txid: &H256) {
    debug!("[remove_unused_tx]|remove old tx|tx_hash:{:}", txid);
    TxFor::<T>::remove(txid);
    InputAddrFor::<T>::remove(txid);
}

pub fn create_multi_address<T: Trait>(
    pubkeys: &Vec<Vec<u8>>,
    sig_num: u32,
    trustee_num: u32,
) -> Option<TrusteeAddrInfo> {
    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + sig_num as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let mut build = Builder::default().push_opcode(opcode);
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

    let addr = Address {
        kind: Type::P2SH,
        network: get_networkid::<T>(),
        hash: dhash160(&redeem_script),
    };
    let script_bytes: Bytes = redeem_script.into();
    Some(TrusteeAddrInfo {
        addr,
        redeem_script: script_bytes.into(),
    })
}

/// Check that the cash withdrawal transaction is correct
pub fn check_withdraw_tx<T: Trait>(tx: &Transaction, withdrawal_id_list: &[u32]) -> Result {
    match Module::<T>::withdrawal_proposal() {
        Some(_) => Err("Unfinished withdrawal transaction"),
        None => {
            // withdrawal addr list for account withdrawal application
            let mut appl_withdrawal_list = Vec::new();
            for withdraw_index in withdrawal_id_list.iter() {
                let record = xrecords::Module::<T>::application_map(withdraw_index)
                    .ok_or("Withdraw id not in withdrawal ApplicationMap record")?;
                // record.data.addr() is base58
                // verify btc address would convert a base58 addr to Address
                let addr: Address = Module::<T>::verify_btc_address(&record.data.addr())
                    .map_err(|_| "Parse addr error")?;
                appl_withdrawal_list.push((addr, record.data.balance().as_() as u64));
            }
            // TODO allow cold address in future, means withdrawal direct from cold address
            let hot_trustee_address: Address = get_hot_trustee_address::<T>()?;
            // withdrawal addr list for tx outputs
            let btc_withdrawal_fee = Module::<T>::btc_withdrawal_fee();
            let mut tx_withdraw_list = Vec::new();
            for output in tx.outputs.iter() {
                let script: Script = output.script_pubkey.clone().into();
                let addr =
                    parse_addr_from_script::<T>(&script).ok_or("not found addr in this out")?;
                if addr.hash != hot_trustee_address.hash {
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
