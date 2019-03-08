use crate::btc_chain::{OutPoint, Transaction};
use crate::btc_keys::{Address, DisplayLayout};
#[cfg(feature = "std")]
use crate::btc_primitives::hash::H256;
use crate::btc_script::{script::Script, ScriptAddress};
use crate::rstd::result::Result;
use crate::{xaccounts, xassets, Module, Trait};

pub fn parse_addr_from_script<T: Trait>(script: &Script) -> Option<Address> {
    let script_addresses = script.extract_destinations().unwrap_or_default();
    // find addr in this transaction
    if script_addresses.len() == 1 {
        let address: &ScriptAddress = &script_addresses[0];
        let net = if Module::<T>::network_id() == 0 {
            keys::Network::Mainnet
        } else {
            keys::Network::Testnet
        };
        let addr = Address {
            kind: address.kind,
            network: net,
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

pub fn get_trustee_address<T: Trait>() -> Result<Address, &'static str> {
    let trustee_address = xaccounts::Module::<T>::trustee_address(xassets::Chain::Bitcoin)
        .ok_or("Should set trustee address first.")?;
    let trustee_hot_address = Address::from_layout(&trustee_address.hot_address.as_slice())
        .map_err(|_| "trustee_address is invalid address")?;
    Ok(trustee_hot_address)
}

#[cfg(feature = "std")]
pub fn hash_strip(hash: &H256) -> String {
    format!("0x{:?}", hash)[..8].to_owned()
}
