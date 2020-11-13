// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_runtime::DispatchResult;
use sp_std::{cmp::Ordering, prelude::Vec};

use light_bitcoin::{
    chain::{OutPoint, Transaction, TransactionOutput},
    keys::{Address, DisplayLayout, Network},
    script::{Opcode, Script, ScriptAddress, ScriptType},
};

use xp_logging::{error, warn};
use xpallet_support::try_hex;

use crate::{native, Error, Module, Trait};

pub fn extract_output_addr(output: &TransactionOutput, network: Network) -> Option<Address> {
    let script = Script::new(output.script_pubkey.clone());

    // only support `p2pk`, `p2pkh` and `p2sh` script
    let script_type = script.script_type();
    match script_type {
        ScriptType::PubKey | ScriptType::PubKeyHash | ScriptType::ScriptHash => {
            let script_addresses = script
                .extract_destinations()
                .map_err(|err| {
                    error!(
                        "[extract_output_addr] Can't extract destinations of btc script err:{}, type:{:?}, script:{}",
                        err, script_type, script
                    );
                }).unwrap_or_default();
            // find address in this transaction
            if script_addresses.len() == 1 {
                let address = &script_addresses[0];
                Some(Address {
                    network,
                    kind: address.kind,
                    hash: address.hash,
                })
            } else {
                warn!(
                    "[extract_output_addr] Can't extract address of btc script, type:{:?}, address:{:?}, script:{}",
                    script_addresses, script_type, script
                );
                None
            }
        }
        _ => None,
    }
}

pub fn is_trustee_addr(addr: Address, trustee_pair: (Address, Address)) -> bool {
    let (hot_addr, cold_addr) = trustee_pair;
    addr.hash == hot_addr.hash || addr.hash == cold_addr.hash
}

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

// Extract the opreturn data from btc null data script.
// OP_RETURN format:
// - op_return + op_push(<0x4c) + data (op_push == data.len())
// - op_return + op_push(=0x4c) + data.len() + data
pub fn extract_opreturn_data(script: &Script) -> Option<Vec<u8>> {
    if !script.is_null_data_script() {
        return None;
    }

    // jump `OP_RETURN`, after checking `is_null_data_script`
    // subscript = `op_push + data` or `op_push + data.len() + data`
    let subscript = script.subscript(1);
    if subscript.is_empty() {
        error!("[parse_opreturn] nothing after `OP_RETURN`, valid in rule but invalid for public consensus");
        return None;
    }

    // parse op_push and data.
    let op_push = subscript[0];
    match op_push.cmp(&(Opcode::OP_PUSHDATA1 as u8)) {
        Ordering::Less => {
            // OP_RETURN format: op_return + op_push(<0x4c) + data (op_push == data.len())
            if subscript.len() < 2 {
                error!(
                    "[parse_opreturn] nothing after `OP_PUSHDATA1`, invalid opreturn script:{:?}",
                    script
                );
                return None;
            }
            let data = &subscript[1..];
            if op_push as usize == data.len() {
                Some(data.to_vec())
            } else {
                error!("[parse_opreturn] unexpected opreturn source error, expected data len:{}, actual data:{:?}", op_push, data);
                None
            }
        }
        Ordering::Equal => {
            // OP_RETURN format: op_return + op_push(=0x4c) + data.len() + data
            //
            // if op_push == `OP_PUSHDATA1`, we must have extra byte for the length of data,
            // otherwise it's an invalid data.
            if subscript.len() < 3 {
                error!(
                    "[parse_opreturn] nothing after `OP_PUSHDATA1`, invalid opreturn script: {:?}",
                    script
                );
                return None;
            }
            let data_len = subscript[1];
            let data = &subscript[2..];
            if data_len as usize == data.len() {
                Some(data.to_vec())
            } else {
                error!("[parse_opreturn] unexpected opreturn source error, expected data len:{}, actual data:{:?}", data_len, data);
                None
            }
        }
        Ordering::Greater => {
            error!(
                "[parse_opreturn] unexpected opreturn source error, \
                opreturn format should be `op_return+op_push+data` or `op_return+op_push+data_len+data`, \
                op_push: {:?}", op_push
            );
            None
        }
    }
}

#[test]
fn test_extract_opreturn_data() {
    // tx: 6b2bea220fdecf30ae3d0e0fa6770f06f281999f81d485ebfc15bdf375268c59
    // null data script: 6a 30 35524745397a4a79667834367934467948444a65317976394e44725946435446746e6e6d714e445077506a6877753871
    let script = "6a3035524745397a4a79667834367934467948444a65317976394e44725946435446746e6e6d714e445077506a6877753871".parse::<Script>().unwrap();
    let data = extract_opreturn_data(&script).unwrap();
    assert_eq!(
        data,
        b"5RGE9zJyfx46y4FyHDJe1yv9NDrYFCTFtnnmqNDPwPjhwu8q".to_vec()
    );

    // tx: 003e7e005b172fe0046fd06a83679fbcdc5e3dd64c8ef9295662a463dea486aa
    // null data script: 6a 38 35515a5947565655507370376362714755634873524a555a726e6d547545796836534c48366a6470667346786770524b404c616f63697573
    let script = "6a3835515a5947565655507370376362714755634873524a555a726e6d547545796836534c48366a6470667346786770524b404c616f63697573".parse::<Script>().unwrap();
    let data = extract_opreturn_data(&script).unwrap();
    assert_eq!(
        data,
        b"5QZYGVVUPsp7cbqGUcHsRJUZrnmTuEyh6SLH6jdpfsFxgpRK@Laocius".to_vec()
    );
}
