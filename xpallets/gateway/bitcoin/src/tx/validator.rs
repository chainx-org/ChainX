// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
};
use sp_std::prelude::Vec;

use light_bitcoin::{chain::{Transaction, TransactionOutput}, keys::{verify_schnorr, AddressTypes, SchnorrSignature, XOnly}, primitives::{Bytes, H256}, script::{check_taproot_tx, Script, ScriptExecutionData, SignatureVersion}};

use xp_logging::{debug, error};

use crate::{trustee::{get_hot_trustee_address, get_hot_trustee_redeem_script}, types::BtcRelayedTx, Error, Trait};
use light_bitcoin::script::TransactionInputSigner;
use sp_std::convert::TryFrom;

pub fn validate_transaction<T: Trait>(
    tx: &BtcRelayedTx,
    merkle_root: H256,
    prev_tx: Option<&Transaction>,
) -> DispatchResult {
    let tx_hash = tx.raw.hash();
    debug!(
        "[validate_transaction] tx_hash:{:?}, relay tx:{:?}",
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
            "[validate_transaction] Check merkle tree proof error, merkle_root:{:?}, hash:{:?}",
            merkle_root, hash
        );
        return Err(Error::<T>::BadMerkleProof.into());
    }
    if !matches.iter().any(|h| *h == tx_hash) {
        error!("[validate_transaction] Tx hash should in matches of partial merkle tree");
        return Err(Error::<T>::BadMerkleProof.into());
    }

    if let Some(prev) = prev_tx {
        // verify prev tx for input
        // only check the first(0) input in transaction
        let previous_txid = prev.hash();
        let expected_id = tx.raw.inputs[0].previous_output.txid;
        if previous_txid != expected_id {
            error!(
                "[validate_transaction] Relay previous tx's hash not equal to relay tx first input, expected_id:{:?}, prev:{:?}",
                expected_id, previous_txid
            );
            return Err(Error::<T>::InvalidPrevTx.into());
        }
    }
    Ok(())
}

/// Check signed transactions
pub fn parse_and_check_signed_tx<T: Trait>(tx: &Transaction) -> Result<u32, DispatchError> {
    let redeem_script = get_hot_trustee_redeem_script::<T>()?;
    parse_and_check_signed_tx_impl::<T>(tx, redeem_script)
}

/// Check Taproot tx
pub fn parse_check_taproot_tx<T: Trait>(tx: &Transaction, spent_outputs: &Vec<TransactionOutput>) -> bool {
    let hot_addr = get_hot_trustee_address::<T>()?;
    let mut script_pubkeys = spent_outputs.iter().map(|d| d.script_pubkey.clone()).collect::<Vec<_>>();
    script_pubkeys.dedup();
    if script_pubkeys.len() != 1 {
        return false;
    }
    let mut keys = [0u8; 32];
    keys.copy_from_slice(&script_pubkey);
    let tweak_pubkey = XOnly(keys);
    if AddressTypes::WitnessV1Taproot(tweak_pubkey) != hot_addr.hash {
        return false;
    }
    check_taproot_tx(tx, spent_outputs)
}

/// for test convenient
#[inline]
pub fn parse_and_check_signed_tx_impl<T: Trait>(
    tx: &Transaction,
    script: Script,
) -> Result<u32, DispatchError> {
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
            input_signs.push(0);
            continue;
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
                error!(
                    "[parse_and_check_signed_tx] Verify sig failed, tx:{:?}, input:{:?}, bytes_redeem_script:{:?}",
                    tx, i, bytes_redeem_script
                );
                return Err(Error::<T>::VerifySignFailed.into());
            }
        }
        input_signs.push(sigs.len());
    }
    // the list length must more than one, due to must have inputs; qed
    ensure!(!input_signs.is_empty(), Error::<T>::InvalidSignCount);

    let first = &input_signs[0];
    // if just one element, `iter().all()` would return true
    if input_signs[1..].iter().all(|item| item == first) {
        Ok(*first as u32)
    } else {
        // all inputs sigs count should be same, otherwise it's an invalid tx
        Err(Error::<T>::InvalidSignCount.into())
    }
}
