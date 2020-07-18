// Copyright 2018-2019 Chainpool.

// Substrate
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
};

use sp_std::{convert::TryFrom, prelude::Vec, result};
// light-bitcoin
use btc_chain::Transaction;
use btc_primitives::{Bytes, H256};
use btc_script::Script;

// use crate::tx::utils::get_hot_trustee_redeem_script;
use crate::types::RelayedTx;
use crate::{Error, Trait};

// ChainX
use xpallet_support::{debug, error, try_hex};

pub fn validate_transaction<T: Trait>(tx: &RelayedTx, merkle_root: H256) -> DispatchResult {
    let tx_hash = tx.raw.hash();
    debug!(
        "[validate_transaction]|txhash:{:}|relay tx:{:?}",
        tx_hash, tx
    );

    // verify merkle proof
    let mut matches = Vec::new();
    let mut _indexes = Vec::new();
    let hash = tx
        .merkle_proof
        .extract_matches(&mut matches, &mut _indexes)
        .map_err(|_| Error::<T>::BadMerkleProof)?;
    if merkle_root != hash {
        error!(
            "[validate_transaction]|Check failed for merkle tree proof|merkle_root:{:?}|hash:{:?}",
            merkle_root, hash
        );
        Err(Error::<T>::BadMerkleProof)?;
    }
    if !matches.iter().any(|h| *h == tx_hash) {
        error!("[validate_transaction]|Tx hash should in matches of partial merkle tree");
        Err(Error::<T>::BadMerkleProof)?;
    }

    // if let Some(prev) = tx.prev_tx() {
    //     // verify prev tx for input
    //     // only check the first(0) input in transaction
    //     let previous_txid = prev.hash();
    //     if previous_txid != tx.raw_tx().inputs[0].previous_output.hash {
    //         error!("[validate_transaction]|relay previou tx's hash not equail to relay tx first input|relaytx:{:?}", tx);
    //         return Err("Previous tx id not equal input point hash");
    //     }
    // }
    Ok(())
}

/// Check signed transactions
// pub fn parse_and_check_signed_tx<T: Trait>(tx: &Transaction) -> result::Result<u32, DispatchError> {
//     let redeem_script = get_hot_trustee_redeem_script::<T>()?;
//     parse_and_check_signed_tx_impl::<T>(tx, redeem_script)
// }

/// for test convenient
#[inline]
pub fn parse_and_check_signed_tx_impl<T: Trait>(
    tx: &Transaction,
    script: Script,
) -> result::Result<u32, DispatchError> {
    let (pubkeys, _, _) = script
        .parse_redeem_script()
        .ok_or(Error::<T>::BadRedeemScript)?;
    let bytes_redeem_script = script.to_bytes();

    let mut input_signs = Vec::new();
    // any input check meet error would return
    for i in 0..tx.inputs.len() {
        // parse sigs from transaction inputs
        let script: Script = tx.inputs[i].script_sig.clone().into();
        if script.len() < 2 {
            // if script length less than 2, it must has no sig in input, use 0 to represent it
            Err(Error::<T>::InvalidSignCount)?;
        }
        let (sigs, _) = script
            .extract_multi_scriptsig()
            .map_err(|_| Error::<T>::BadSignature)?;

        for sig in sigs.iter() {
            let verify = pubkeys.iter().any(|pubkey| {
                super::secp256k1_verifier::verify_sig::<T>(sig, pubkey, tx, &bytes_redeem_script, i)
                    .is_ok()
            });
            if !verify {
                error!("[parse_and_check_signed_tx]|Verify sign failed|tx:{:?}|input:{:?}|bytes_sedeem_script:{:?}", tx, i, try_hex!(&bytes_redeem_script));
                Err(Error::<T>::VerifySignFailed)?
            }
        }
        input_signs.push(sigs.len());
    }
    // the list length must more than one, due to must have inputs; qed
    ensure!(input_signs.len() > 0, Error::<T>::InvalidSignCount);

    let first = &input_signs[0];
    // if just one element, `iter().all()` would return true
    if input_signs[1..].iter().all(|item| item == first) {
        Ok(*first as u32)
    } else {
        // all inputs sigs count should be same, otherwise it's an invalid tx
        Err(Error::<T>::InvalidSignCount)?
    }
}
