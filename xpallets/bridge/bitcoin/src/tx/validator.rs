// Copyright 2018-2019 Chainpool.

// Substrate
use primitives::traits::MaybeDebug;
use sp_std::{prelude::Vec, result};
use support::dispatch::Result;
// light-bitcoin
use btc_chain::Transaction;
use btc_keys::Public;
use btc_primitives::{Bytes, H256};
use btc_script::{
    Script, SignatureChecker, SignatureVersion, TransactionInputSigner, TransactionSignatureChecker,
};

use crate::traits::RelayTransaction;
use crate::tx::utils::get_hot_trustee_redeem_script;
use crate::Trait;

// ChainX
#[cfg(feature = "std")]
use xsupport::as_string_hex;
use xsupport::{debug, error};

pub fn validate_transaction<T: Trait, RT: RelayTransaction + MaybeDebug>(
    tx: &RT,
    merkle_root: H256,
) -> Result {
    let tx_hash = tx.tx_hash();
    debug!(
        "[validate_transaction]|txhash:{:}|relay tx:{:?}",
        tx_hash, tx
    );

    // verify merkle proof
    let mut matches = Vec::new();
    let mut _indexes = Vec::new();
    match tx
        .merkle_proof()
        .extract_matches(&mut matches, &mut _indexes)
    {
        Ok(hash) => {
            if merkle_root != hash {
                return Err("Check failed for merkle tree proof");
            }
            if !matches.iter().any(|h| *h == tx_hash) {
                return Err("Tx hash should in matches of partial merkle tree");
            }
        }
        Err(_) => return Err("Parse partial merkle tree failed"),
    }

    if let Some(prev) = tx.prev_tx() {
        // verify prev tx for input
        // only check the first(0) input in transaction
        let previous_txid = prev.hash();
        if previous_txid != tx.raw_tx().inputs[0].previous_output.hash {
            error!("[validate_transaction]|relay previou tx's hash not equail to relay tx first input|relaytx:{:?}", tx);
            return Err("Previous tx id not equal input point hash");
        }
    }
    Ok(())
}

fn verify_sig(
    sig: &Bytes,
    pubkey: &Bytes,
    tx: &Transaction,
    script_pubkey: &Bytes,
    index: usize,
) -> bool {
    let tx_signer: TransactionInputSigner = tx.clone().into();
    // TODO WARNNING!!! when support WitnessV0, the `input_amount` must set value
    let checker = TransactionSignatureChecker {
        input_index: index,
        input_amount: 0,
        signer: tx_signer,
    };
    let sighashtype = 1; // Sighsh all
    let signature = sig.clone().take().into();
    let public = if let Ok(public) = Public::from_slice(pubkey.as_slice()) {
        public
    } else {
        return false;
    };

    //privous tx's output script_pubkey
    let script_code: Script = script_pubkey.clone().into();
    return checker.check_signature(
        &signature,
        &public,
        &script_code,
        sighashtype,
        SignatureVersion::Base,
    );
}

/// Check signed transactions
pub fn parse_and_check_signed_tx<T: Trait>(tx: &Transaction) -> result::Result<u32, &'static str> {
    let redeem_script = get_hot_trustee_redeem_script::<T>()?;
    parse_and_check_signed_tx_impl(tx, redeem_script)
}

/// for test convenient
#[inline]
pub fn parse_and_check_signed_tx_impl(
    tx: &Transaction,
    script: Script,
) -> result::Result<u32, &'static str> {
    let (pubkeys, _, _) = script
        .parse_redeem_script()
        .ok_or("Parse redeem script failed")?;
    let bytes_redeem_script = script.to_bytes();

    let mut v = Vec::new();
    // any input check meet error would return
    for i in 0..tx.inputs.len() {
        // parse sigs from transaction inputs
        let script: Script = tx.inputs[i].script_sig.clone().into();
        if script.len() < 2 {
            // if script length less than 2, it must has no sig in input, use 0 to represent it
            v.push(0);
            continue;
        }
        let (sigs, _) = script
            .extract_multi_scriptsig()
            .map_err(|_| "Invalid signature")?;

        for sig in sigs.iter() {
            let mut verify = false;
            for pubkey in pubkeys.iter() {
                if verify_sig(sig, pubkey, tx, &bytes_redeem_script, i) {
                    verify = true;
                    break;
                }
            }
            if !verify {
                error!("[parse_and_check_signed_tx]|Verify sign failed|tx:{:?}|input:{:?}|bytes_sedeem_script:{:?}", tx, i, as_string_hex(&bytes_redeem_script));
                return Err("Verify sign failed");
            }
        }
        v.push(sigs.len());
    }
    assert!(
        v.len() > 0,
        "the list length must more than one, due to must have inputs; qed"
    );

    assert!(
        v.len() == tx.inputs.len(),
        "the list length must equal to inputs counts; qed"
    );

    let first = v.get(0).unwrap();
    if v[1..].iter().all(|item| item == first) {
        Ok(*first as u32)
    } else {
        Err("all inputs sigs count should be same, otherwise it's an invalid tx")
    }
}
