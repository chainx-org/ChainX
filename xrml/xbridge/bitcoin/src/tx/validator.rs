use super::*;

use super::keys::Public;
use super::{
    Bytes, Result, Script, SignatureChecker, SignatureVersion, StorageMap, Trait, Transaction,
    TransactionInputSigner, TransactionSignatureChecker, TrusteeRedeemScript,
};
use chain::Transaction as BTCTransaction;
use codec::Decode;

pub fn validate_transaction<T: Trait>(
    tx: &RelayTx,
    address: &keys::Address,
) -> StdResult<TxType, &'static str> {
    let verify_txid = tx.raw.hash();
    match <BlockHeaderFor<T>>::get(&tx.block_hash) {
        Some(header) => {
            let mut itervc = header.txid.iter();
            if itervc.any(|h| *h == verify_txid) {
                return Err("This tx already store");
            }
        }
        None => return Err("Can't find this tx's block header"),
    }

    let header_info = match <BlockHeaderFor<T>>::get(&tx.block_hash) {
        Some(header) => header,
        None => return Err("This block header not exists"),
    };

    let merkle_root = header_info.header.merkle_root_hash;

    // Verify proof
    match parse_partial_merkle_tree(tx.merkle_proof.clone()) {
        Ok(parsed) => {
            if merkle_root != parsed.root {
                return Err("Check failed for merkle tree proof");
            }
            if !parsed.hashes.iter().any(|h| *h == verify_txid) {
                return Err("Tx hash should in ParsedPartialMerkleTree");
            }
        }
        Err(_) => return Err("Parse partial merkle tree failed"),
    }

    // To do: All inputs relay
    let previous_txid = tx.previous_raw.hash();
    if previous_txid != tx.raw.inputs[0].previous_output.hash {
        return Err("Previous tx id not right");
    }

    // detect withdraw: To do: All inputs relay
    let outpoint = tx.raw.inputs[0].previous_output.clone();
    let send_address = match inspect_address::<T>(&tx.previous_raw, outpoint) {
        Some(a) => a,
        None => return Err("Inspect address failed at detect withdraw-tx "),
    };
    if send_address.hash == address.hash {
        return Ok(TxType::Withdraw);
    }

    // detect deposit
    for output in tx.raw.outputs.iter() {
        if is_key(&output.script_pubkey, &address) {
            return Ok(TxType::BindDeposit);
        }
    }

    Err("Irrelevant tx")
}

fn verify_sign(sign: &Bytes, pubkey: &Bytes, tx: &Transaction, script_pubkey: &Bytes) -> bool {
    let tx_signer: TransactionInputSigner = tx.clone().into();
    let checker = TransactionSignatureChecker {
        input_index: 0,
        input_amount: 0,
        signer: tx_signer,
    };
    let sighashtype = 1; // Sighsh all
    let signature = sign.clone().take().into();
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

pub fn handle_condidate<T: Trait>(tx: Vec<u8>) -> Result {
    let tx: BTCTransaction = Decode::decode(&mut tx.as_slice()).ok_or("Parse transaction err")?;
    let trustee_info =
        <TrusteeRedeemScript<T>>::get().ok_or("Should set trustee address info first.")?;
    let redeem_script = Script::from(trustee_info.hot_redeem_script);
    let script: Script = tx.inputs[0].script_sig.clone().into();
    if script.len() < 2 {
        return Err("Invalid signature, script_sig is too short");
    }
    let (sigs, _) = if let Ok((sigs, s)) = script.extract_multi_scriptsig() {
        (sigs, s)
    } else {
        return Err("Invalid signature");
    };

    let (pubkeys, _, _) = match redeem_script.parse_redeem_script() {
        Some((k, s, l)) => (k, s, l),
        None => return Err("Parse redeem script failed"),
    };
    for sig in sigs.clone() {
        let mut verify = false;
        for pubkey in pubkeys.clone() {
            if verify_sign(&sig, &pubkey, &tx, &redeem_script.to_bytes()) {
                verify = true;
                break;
            }
        }
        if !verify {
            return Err("Verify sign failed");
        }
    }

    Ok(())
}
