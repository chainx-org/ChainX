// Copyright 2018-2019 Chainpool.
// Substrate
use rstd::prelude::Vec;
use rstd::result::Result;

// ChainX
use xbridge_common::{traits::TrusteeSession, types::TrusteeSessionInfo, utils::two_thirds_unsafe};
use xsupport::error;

// light-bitcoin
use btc_chain::{OutPoint, Transaction};
use btc_keys::{Address, Network};
use btc_script::{Opcode, Script, ScriptAddress};

use crate::types::TrusteeAddrInfo;
use crate::{Module, Trait};

pub fn get_networkid<T: Trait>() -> Network {
    if Module::<T>::network_id() == 0 {
        Network::Mainnet
    } else {
        Network::Testnet
    }
}

pub fn parse_addr_from_script<T: Trait>(script: &Script) -> Option<Address> {
    let script_addresses = script.extract_destinations().unwrap_or_default();
    // find addr in this transaction
    if script_addresses.len() == 1 {
        let address: &ScriptAddress = &script_addresses[0];
        let addr = Address {
            kind: address.kind,
            network: get_networkid::<T>(),
            hash: address.hash.clone(), // public key hash
        };
        return Some(addr);
    }
    None
}

pub fn parse_opreturn(script: &Script) -> Option<Vec<u8>> {
    if script.is_null_data_script() {
        // jump OP_RETURN, when after `is_null_data_script`, subscript must larger and equal than 1
        let s = script.subscript(1);
        if s.len() == 0 {
            error!("[parse_opreturn]|nothing after `OP_RETURN`, valid in rule but not valid for public consensus");
            return None;
        }
        // script must large then 1
        if s[0] < Opcode::OP_PUSHDATA1 as u8 {
            if s[0] as usize == (&s[1..]).len() {
                return Some(s[1..].to_vec());
            } else {
                error!("[parse_opreturn]|unexpect! opreturn source error, len not equal to real len|len:{:?}|real:{:?}", s[0], &s[1..]);
                return None;
            }
        } else if s[0] == Opcode::OP_PUSHDATA1 as u8 {
            // when subscript [0] is `OP_PUSHDATA1`, must have [1], or is an invalid data
            if s.len() < 2 {
                error!(
                    "[parse_opreturn]|nothing after `OP_PUSHDATA1`, not a valid opreturn|{:?}",
                    s
                );
                return None;
            }
            // script must large then 2
            if s[1] as usize == (&s[2..]).len() {
                return Some(s[2..].to_vec());
            } else {
                error!("[parse_opreturn]|unexpect! opreturn source error, len not equal to real len|len mark:{:?}|len:{:?}|real:{:?}", s[0], s[1], &s[2..]);
                return None;
            }
        } else {
            error!("[parse_opreturn]|unexpect! opreturn source error, opreturn should not");
            None
        }
    } else {
        // do nothing
        None
    }
}

/// parse addr from a transaction output
pub fn inspect_address_from_transaction<T: Trait>(
    tx: &Transaction,
    outpoint: &OutPoint,
) -> Option<Address> {
    tx.outputs
        .get(outpoint.index as usize)
        .map(|output| {
            let script: Script = (*output).script_pubkey.clone().into();
            script
        })
        .and_then(|script| parse_addr_from_script::<T>(&script))
}

/// judge a script's addr is equal to second param
pub fn is_key<T: Trait>(script: &Script, trustee_address: &Address) -> bool {
    if let Some(addr) = parse_addr_from_script::<T>(script) {
        if addr.hash == trustee_address.hash {
            return true;
        }
    }
    false
}

#[inline]
pub fn equal_addr(addr1: &Address, addr2: &Address) -> bool {
    addr1.hash == addr2.hash
}

pub fn trustee_session<T: Trait>(
) -> Result<TrusteeSessionInfo<T::AccountId, TrusteeAddrInfo>, &'static str> {
    T::TrusteeSessionProvider::current_trustee_session()
}

#[inline]
fn trustee_addr_info_pair<T: Trait>() -> Result<(TrusteeAddrInfo, TrusteeAddrInfo), &'static str> {
    T::TrusteeSessionProvider::current_trustee_session()
        .map(|session_info| (session_info.hot_address, session_info.cold_address))
}

#[inline]
pub fn get_trustee_address_pair<T: Trait>() -> Result<(Address, Address), &'static str> {
    trustee_addr_info_pair::<T>().map(|(hot_info, cold_info)| (hot_info.addr, cold_info.addr))
}

#[inline]
pub fn get_last_trustee_address_pair<T: Trait>() -> Result<(Address, Address), &'static str> {
    T::TrusteeSessionProvider::last_trustee_session().map(|session_info| {
        (
            session_info.hot_address.addr,
            session_info.cold_address.addr,
        )
    })
}

pub fn get_hot_trustee_address<T: Trait>() -> Result<Address, &'static str> {
    trustee_addr_info_pair::<T>().map(|(addr_info, _)| addr_info.addr)
}

pub fn get_hot_trustee_redeem_script<T: Trait>() -> Result<Script, &'static str> {
    trustee_addr_info_pair::<T>().map(|(addr_info, _)| addr_info.redeem_script.into())
}

/// Get the required number of signatures
/// sig_num: Number of signatures required
/// trustee_num: Total number of multiple signatures
/// NOTE: Signature ratio greater than 2/3
pub fn get_sig_num<T: Trait>() -> (u32, u32) {
    let trustee_list = T::TrusteeSessionProvider::current_trustee_session()
        .map(|session_info| session_info.trustee_list)
        .expect("the trustee_list must exist; qed");
    let trustee_num = trustee_list.len() as u32;
    (two_thirds_unsafe(trustee_num), trustee_num)
}

pub fn ensure_identical(tx1: &Transaction, tx2: &Transaction) -> Result<(), &'static str> {
    if tx1.version == tx2.version
        && tx1.outputs == tx2.outputs
        && tx1.lock_time == tx2.lock_time
        && tx1.inputs.len() == tx2.inputs.len()
    {
        for i in 0..tx1.inputs.len() {
            if tx1.inputs[i].previous_output != tx2.inputs[i].previous_output
                || tx1.inputs[i].sequence != tx2.inputs[i].sequence
            {
                error!(
                    "[ensure_identical]|tx1 not equal to tx2|tx1:{:?}|tx2:{:?}",
                    tx1, tx2
                );
                return Err("The inputs of these two transactions mismatch.");
            }
        }
        return Ok(());
    }
    Err("The transaction text does not match the original text to be signed.")
}
