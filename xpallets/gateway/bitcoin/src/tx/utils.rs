// Copyright 2018-2019 Chainpool.
// Substrate
use frame_support::{debug::native, dispatch::DispatchResult};
use sp_std::prelude::Vec;

// ChainX
// use xbridge_common::{traits::TrusteeSession, types::TrusteeSessionInfo, utils::two_thirds_unsafe};
use xpallet_support::{base58::to_base58, error, try_hex, warn, RUNTIME_TARGET};

// light-bitcoin
use btc_chain::{OutPoint, Transaction};
use btc_keys::{Address, DisplayLayout, Network};
use btc_script::{Opcode, Script, ScriptAddress};

use crate::{Error, Module, Trait};

pub fn parse_output_addr<T: Trait>(script: &Script) -> Option<Address> {
    let network = Module::<T>::network_id();
    parse_output_addr_with_networkid(script, network)
}

pub fn parse_output_addr_with_networkid(script: &Script, network: Network) -> Option<Address> {
    // only `p2pk`, `p2pkh`, `p2sh` could parse
    script.extract_destinations().map_err(|_e|{
        error!(
            "[parse_output_addr]|parse output script error|e:{:}|script:{:?}",
            _e,
            try_hex!(&script)
        );
        _e
    }).ok().and_then(|script_addresses| {
        // find addr in this transaction
        if script_addresses.len() == 1 {
            let address: &ScriptAddress = &script_addresses[0];
            let addr = Address {
                kind: address.kind,
                network: network,
                hash: address.hash.clone(), // public key hash
            };
            return Some(addr);
        }
        // the type is `NonStandard`, `Multisig`, `NullData`, `WitnessScript`, `WitnessKey`
        warn!("[parse_output_addr]|can't parse addr from output script|type:{:?}|addr:{:?}|script:{:?}", script.script_type(), script_addresses, try_hex!(&script));
        None
    })
}

/// parse addr from a transaction output, getting addr from prev_tx output
/// notice, only can parse `p2pk`, `p2pkh`, `p2sh` output,
/// other type would return None
pub fn inspect_address_from_transaction(
    tx: &Transaction,
    outpoint: &OutPoint,
    network: Network,
) -> Option<Address> {
    tx.outputs
        .get(outpoint.index as usize)
        .map(|output| {
            let script: Script = (*output).script_pubkey.clone().into();
            script
        })
        .and_then(|script| parse_output_addr_with_networkid(&script, network))
}

/// judge a script's addr is equal to second param
pub fn is_key(script: &Script, trustee_address: &Address, network: Network) -> bool {
    if let Some(addr) = parse_output_addr_with_networkid(script, network) {
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

pub fn ensure_identical<T: Trait>(tx1: &Transaction, tx2: &Transaction) -> DispatchResult {
    if tx1.version == tx2.version
        && tx1.outputs == tx2.outputs
        && tx1.lock_time == tx2.lock_time
        && tx1.inputs.len() == tx2.inputs.len()
    {
        for i in 0..tx1.inputs.len() {
            if tx1.inputs[i].previous_output != tx2.inputs[i].previous_output
                || tx1.inputs[i].sequence != tx2.inputs[i].sequence
            {
                native::error!(
                    target: RUNTIME_TARGET,
                    "[ensure_identical]|tx1 not equal to tx2|tx1:{:?}|tx2:{:?}",
                    tx1,
                    tx2
                );
                Err(Error::<T>::MismatchedTx)?;
            }
        }
        return Ok(());
    }
    native::error!(
        target: RUNTIME_TARGET,
        "The transaction text does not match the original text to be signed",
    );
    Err(Error::<T>::MismatchedTx)?
}

#[inline]
pub fn addr2vecu8(addr: &Address) -> Vec<u8> {
    to_base58(addr.layout().to_vec())
}

#[cfg(feature = "std")]
#[inline]
pub fn trick_format_opreturn(opreturn: &[u8]) -> String {
    if opreturn.len() > 2 {
        // trick, just for print log
        format!("{:?}|{:?}", &opreturn[..2], try_hex!(&opreturn[2..]))
    } else {
        format!("{:?}", opreturn)
    }
}
