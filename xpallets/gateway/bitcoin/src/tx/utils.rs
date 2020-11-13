// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_runtime::DispatchResult;
use sp_std::prelude::Vec;

use light_bitcoin::{
    chain::Transaction,
    keys::{Address, DisplayLayout},
};

use crate::{native, Error, Trait};

/*
pub fn parse_output_addr<T: Trait>(script: &Script) -> Option<Address> {
    let network = Module::<T>::network_id();
    parse_output_addr_with_networkid(script, network)
}

pub fn parse_output_addr_with_networkid(script: &Script, network: Network) -> Option<Address> {
    // only `p2pk`, `p2pkh`, `p2sh` could parse
    script.extract_destinations().map_err(|err|{
        error!(
            "[parse_output_addr] Parse output script error:{}, script:{:?}",
            err,
            try_hex!(&script)
        );
        err
    }).ok().and_then(|script_addresses| {
        // find addr in this transaction
        if script_addresses.len() == 1 {
            let address: &ScriptAddress = &script_addresses[0];
            let addr = Address {
                kind: address.kind,
                network,
                hash: address.hash, // public key hash
            };
            return Some(addr);
        }
        // the type is `NonStandard`, `Multisig`, `NullData`, `WitnessScript`, `WitnessKey`
        warn!(
            "[parse_output_addr] Can not parse addr from output script, type:{:?}, addr:{:?}, script:{:?}",
            script.script_type(), script_addresses, try_hex!(&script)
        );
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
*/

/// Returns Ok if `tx1` and `tx2` are the same transaction.
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
                native!(
                    error,
                    "[ensure_identical] Tx1 is different to Tx2, tx1:{:?}, tx2:{:?}",
                    tx1,
                    tx2
                );
                return Err(Error::<T>::MismatchedTx.into());
            }
        }
        return Ok(());
    }
    native!(
        error,
        "The transaction text does not match the original text to be signed",
    );
    Err(Error::<T>::MismatchedTx.into())
}

#[inline]
pub fn addr2vecu8(addr: &Address) -> Vec<u8> {
    bs58::encode(&*addr.layout()).into_vec()
}

/*
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
*/
