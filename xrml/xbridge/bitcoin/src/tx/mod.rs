// Copyright 2018-2019 Chainpool.

pub mod handler;
pub mod utils;
mod validator;

// Substrate
use primitives::traits::As;
use rstd::{prelude::*, result};
use support::{dispatch::Result, StorageMap};

// ChainX
use xsupport::{debug, error};

// light-bitcoin
use btc_chain::Transaction;
use btc_crypto::dhash160;
use btc_keys::{Address, Public, Type};
use btc_primitives::{Bytes, H256};
use btc_script::{Builder, Opcode, Script};

use crate::types::{RelayTx, TrusteeAddrInfo, TxType};
use crate::{InputAddrFor, Module, RawEvent, Trait, TxFor};

use self::handler::TxHandler;
use self::utils::{
    equal_addr, get_hot_trustee_address, get_last_trustee_address_pair, get_networkid,
    get_trustee_address_pair, inspect_address_from_transaction, parse_addr_from_script,
};
pub use self::validator::{parse_and_check_signed_tx, validate_transaction};

pub fn detect_transaction_type<T: Trait>(
    relay_tx: &RelayTx,
) -> result::Result<(TxType, Option<Address>), &'static str> {
    let addr_pair = get_trustee_address_pair::<T>()?;
    let last_addr_pair = get_last_trustee_address_pair::<T>()
        .map_err(|_e| {
            error!(
                "[detect_transaction_type]|get_last_trustee_address_pair|err:{:?}",
                _e
            );
            _e
        })
        .ok();
    detect_transaction_type_impl::<T>(relay_tx, addr_pair, last_addr_pair)
}

/// parse tx's inputs/outputs into Option<Address>
/// e.g
/// notice the relay tx only has first input
///        _________
///  addr |        | Some(addr)
///       |   tx   | Some(addr)
///       |________| None (OP_RETURN or something unknown)
/// then judge type
/// when type is deposit, would return Option<Addr> for this deposit input_addr
#[inline]
pub fn detect_transaction_type_impl<T: Trait>(
    relay_tx: &RelayTx,
    trustee_addr_pair: (Address, Address),
    old_trustee_addr_pair: Option<(Address, Address)>,
) -> result::Result<(TxType, Option<Address>), &'static str> {
    let (hot_addr, cold_addr) = trustee_addr_pair;
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
            return Ok((TxType::HotAndCold, None));
        }
        // outputs contains other addr, it's user addr, thus it's a withdrawal
        return Ok((TxType::Withdrawal, None));
    } else {
        if let Some((old_hot_addr, old_cold_addr)) = old_trustee_addr_pair {
            let input_is_old_trustee =
                equal_addr(&input_addr, &old_hot_addr) || equal_addr(&input_addr, &old_cold_addr);
            if input_is_old_trustee && all_outputs_trustee {
                // input should from old trustee addr, outputs should all be current trustee addrs
                return Ok((TxType::TrusteeTransition, None));
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
            return Ok((TxType::Deposit, Some(input_addr)));
        }
    }

    Ok((TxType::Irrelevance, None))
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
    pubkeys: &Vec<Public>,
    sig_num: u32,
) -> Option<TrusteeAddrInfo> {
    let sum = pubkeys.len() as u32;
    if sig_num > sum {
        panic!("required sig num should less than trustee_num; qed")
    }
    if sum > 15 {
        error!("bitcoin's multisig can't more than 15, current is:{:}", sum);
        return None;
    }

    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + sig_num as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let mut build = Builder::default().push_opcode(opcode);
    for pubkey in pubkeys.iter() {
        build = build.push_bytes(&pubkey);
    }

    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + sum as u8 - 1) {
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
            // not allow deposit directly to cold address, only hot address allow
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
/// only allow insert once
pub fn insert_trustee_vote_state<T: Trait>(
    state: bool,
    who: &T::AccountId,
    trustee_list: &mut Vec<(T::AccountId, bool)>,
) -> Result {
    match trustee_list.iter_mut().find(|ref info| info.0 == *who) {
        Some(_) => {
            // if account is exist, override state
            error!("[insert_trustee_vote_state]|already vote for this withdrawal proposal|who:{:?}|old vote:{:}", who, state);
            return Err("already vote for this withdrawal proposal");
        }
        None => {
            trustee_list.push((who.clone(), state));
            debug!(
                "[insert_trustee_vote_state]|insert new vote|who:{:?}|state:{:}",
                who, state
            );
        }
    }
    Module::<T>::deposit_event(RawEvent::SignWithdrawalProposal(who.clone(), state));
    Ok(())
}
