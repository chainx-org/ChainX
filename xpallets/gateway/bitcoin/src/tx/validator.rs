// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    log::{debug, error},
};
use sp_std::prelude::Vec;

use light_bitcoin::{
    chain::{Transaction, TransactionOutputArray},
    keys::{AddressTypes, XOnly},
    primitives::H256,
    script::Script,
};

use crate::{trustee::get_hot_trustee_address, types::BtcRelayedTx, Config, Error};

pub fn validate_transaction<T: Config>(
    tx: &BtcRelayedTx,
    merkle_root: H256,
    prev_tx: Option<&Transaction>,
) -> DispatchResult {
    let tx_hash = tx.raw.hash();
    debug!(
        target: "runtime::bitcoin",
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
            target: "runtime::bitcoin",
            "[validate_transaction] Check merkle tree proof error, merkle_root:{:?}, hash:{:?}",
            merkle_root, hash
        );
        return Err(Error::<T>::BadMerkleProof.into());
    }
    if !matches.iter().any(|h| *h == tx_hash) {
        error!(
            target: "runtime::bitcoin",
            "[validate_transaction] Tx hash should in matches of partial merkle tree"
        );
        return Err(Error::<T>::BadMerkleProof.into());
    }

    if let Some(prev) = prev_tx {
        // verify prev tx for input
        // only check the first(0) input in transaction
        let previous_txid = prev.hash();
        let expected_id = tx.raw.inputs[0].previous_output.txid;
        if previous_txid != expected_id {
            error!(
                target: "runtime::bitcoin",
                "[validate_transaction] Relay previous tx's hash not equal to relay tx first input, expected_id:{:?}, prev:{:?}",
                expected_id, previous_txid
            );
            return Err(Error::<T>::InvalidPrevTx.into());
        }
    }
    Ok(())
}

/// Check Taproot tx
#[allow(dead_code)]
pub fn parse_check_taproot_tx<T: Config>(
    _tx: &Transaction,
    spent_outputs: &TransactionOutputArray,
) -> Result<bool, DispatchError> {
    let hot_addr = get_hot_trustee_address::<T>()?;
    let mut script_pubkeys = spent_outputs
        .outputs
        .iter()
        .map(|d| d.script_pubkey.clone())
        .collect::<Vec<_>>();
    script_pubkeys.dedup();
    if script_pubkeys.len() != 1 {
        return Err(Error::<T>::InvalidPublicKey.into());
    }

    let script: Script = script_pubkeys[0].clone().into();

    if !script.is_pay_to_witness_taproot() {
        return Err(Error::<T>::InvalidPublicKey.into());
    }

    let mut keys = [0u8; 32];
    keys.copy_from_slice(&script_pubkeys[0][2..]);
    let tweak_pubkey = XOnly(keys);
    if AddressTypes::WitnessV1Taproot(tweak_pubkey) != hot_addr.hash {
        return Err(Error::<T>::InvalidPublicKey.into());
    }
    // Some data types cannot implement codec encode decode
    // if light_bitcoin::script::check_taproot_tx(tx, &spent_outputs.outputs).is_err() {
    //     Err(Error::<T>::VerifySignFailed.into())
    // } else {
    //     Ok(true)
    // }

    Ok(true)
}
