// Copyright 2018-2019 Chainpool.

use parity_codec::Decode;

// Substrate
use rstd::result::Result;

// ChainX
use xsupport::error;
#[cfg(feature = "std")]
use xsupport::u8array_to_addr;

// light-bitcoin
use btc_chain::{OutPoint, Transaction};
use btc_keys::{Address, Network};
use btc_script::{Script, ScriptAddress};

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

#[inline]
fn trustee_addr_info<T: Trait>(is_hot: bool) -> Result<TrusteeAddrInfo, &'static str> {
    let (hot_address, cold_address) =
        xaccounts::Module::<T>::trustee_address(xassets::Chain::Bitcoin)
            .ok_or("Should set trustee address first.")?;

    let addr_info: TrusteeAddrInfo = if is_hot {
        Decode::decode(&mut hot_address.as_slice()).ok_or_else(|| {
            error!(
                "[trustee_addr_info]|parse hot trustee addr info error|src:{:}",
                u8array_to_addr(&hot_address)
            );
            "parse hot trustee addr info error"
        })?
    } else {
        Decode::decode(&mut cold_address.as_slice()).ok_or_else(|| {
            error!(
                "[trustee_addr_info]|parse cold trustee addr info error|src:{:}",
                u8array_to_addr(&hot_address)
            );
            "parse cold trustee addr info error"
        })?
    };
    Ok(addr_info)
}

pub fn get_hot_trustee_address<T: Trait>() -> Result<Address, &'static str> {
    trustee_addr_info::<T>(true).map(|addr_info| addr_info.addr)
}

pub fn get_hot_trustee_redeem_script<T: Trait>() -> Result<Script, &'static str> {
    trustee_addr_info::<T>(true).map(|addr_info| addr_info.redeem_script.into())
}

// pub fn get_cold_trustee_address<T: Trait>() -> Result<Address, &'static str> {
//     trustee_addr_info::<T>(false).map(|addr_info| addr_info.addr)
// }
//
// pub fn get_cold_trustee_redeem_script<T: Trait>() -> Result<Script, &'static str> {
//     trustee_addr_info::<T>(false).map(|addr_info| addr_info.redeem_script.into())
// }

pub fn get_trustee_address_pair<T: Trait>(
    session: u32,
) -> Result<(Address, Address), &'static str> {
    let (hot_address, cold_address) =
        xaccounts::Module::<T>::trustee_address_of(xassets::Chain::Bitcoin, session)
            .ok_or("Should set trustee address first.")?;

    let hot_info: TrusteeAddrInfo =
        Decode::decode(&mut hot_address.as_slice()).ok_or_else(|| {
            error!(
                "[get_trustee_address_pair]|parse hot trustee addr info error|src:{:}",
                u8array_to_addr(&hot_address)
            );
            "parse hot trustee addr info error"
        })?;
    let cold_info: TrusteeAddrInfo =
        Decode::decode(&mut cold_address.as_slice()).ok_or_else(|| {
            error!(
                "[get_trustee_address_pair]|parse cold trustee addr info error|src:{:}",
                u8array_to_addr(&hot_address)
            );
            "parse cold trustee addr info error"
        })?;
    Ok((hot_info.addr, cold_info.addr))
}

/// Get the required number of signatures
/// sig_num: Number of signatures required
/// trustee_num: Total number of multiple signatures
/// NOTE: Signature ratio greater than 2/3
pub fn get_sig_num<T: Trait>() -> (u32, u32) {
    let trustee_list = xaccounts::Module::<T>::trustee_list(xassets::Chain::Bitcoin)
        .expect("the trustee_list must exist; qed");
    let trustee_num = trustee_list.len() as u32;
    get_sig_num_from_trustees(trustee_num)
}

#[inline]
pub fn get_sig_num_from_trustees(trustee_num: u32) -> (u32, u32) {
    let sig_num = match 2_u32.checked_mul(trustee_num) {
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
