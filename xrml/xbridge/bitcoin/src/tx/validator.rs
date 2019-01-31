use super::*;

use super::keys::{DisplayLayout, Public};
use super::{
    Bytes, Result, Script, SignatureChecker, SignatureVersion, StorageMap, Trait, Transaction,
    TransactionInputSigner, TransactionSignatureChecker,
};

pub fn validate_transaction<T: Trait>(
    tx: &RelayTx,
    address: &keys::Address,
) -> StdResult<TxType, &'static str> {
    let verify_txid = tx.raw.hash();
    match <BlockHeaderFor<T>>::get(&tx.block_hash) {
        Some(header) => {
            let mut itervc = header.txid.iter();
            if itervc.any(|h| *h == verify_txid) {
                return Err("this tx already store");
            }
        }
        None => return Err("can't find this tx's block header"),
    }

    let header_info = match <BlockHeaderFor<T>>::get(&tx.block_hash) {
        Some(header) => header,
        None => return Err("not has this block header yet"),
    };

    let merkle_root = header_info.header.merkle_root_hash;

    // Verify proof
    match parse_partial_merkle_tree(tx.merkle_proof.clone()) {
        Ok(parsed) => {
            if merkle_root != parsed.root {
                return Err("check failed for merkle tree proof");
            }
            if !parsed.hashes.iter().any(|h| *h == verify_txid) {
                return Err("txid should in ParsedPartialMerkleTree");
            }
        }
        Err(_) => return Err("parse partial merkle tree failed"),
    }

    // To do: All inputs relay
    let previous_txid = tx.previous_raw.hash();
    if previous_txid != tx.raw.inputs[0].previous_output.hash {
        return Err("previous tx id not right");
    }

    // detect withdraw: To do: All inputs relay
    let outpoint = tx.raw.inputs[0].previous_output.clone();
    let send_address = match inspect_address::<T>(&tx.previous_raw, outpoint) {
        Some(a) => a,
        None => return Err("inspect address failed at detect withdraw-tx "),
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

    Err("not found our pubkey, may be an unrelated tx")
}

fn verify_sign(sign: &Bytes, pubkey: &Bytes, tx: &Transaction, script_pubkey: &Bytes) -> bool {
    let tx_signer: TransactionInputSigner = tx.clone().into();
    let checker = TransactionSignatureChecker {
        input_index: 0,
        input_amount: 0,
        signer: tx_signer,
    };
    let sighashtype = 0x41; // Sighsh all
    let signature = sign.clone().take().into();
    let public = if let Ok(public) = Public::from_slice(&pubkey) {
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

pub fn handle_condidate<T: Trait>(tx: Transaction) -> Result {
    let trustee_address = <xaccounts::TrusteeAddress<T>>::get(xassets::Chain::Bitcoin)
        .ok_or("Should set RECEIVE_address first.")?;
    let hot_address = Address::from_layout(&trustee_address.hot_address.as_slice())
        .map_err(|_| "Invalid Address")?;
    let pk = hot_address.hash.clone().to_vec();
    let mut script_pubkey = Bytes::new();
    script_pubkey.push(Opcode::OP_HASH160 as u8);
    script_pubkey.push(Opcode::OP_PUSHBYTES_20 as u8);
    for p in pk {
        script_pubkey.push(p)
    }
    script_pubkey.push(Opcode::OP_EQUAL as u8);

    let script: Script = tx.inputs[0].script_sig.clone().into();
    let (sigs, _dem) = if let Ok((sigs, dem)) = script.extract_multi_scriptsig() {
        (sigs, dem)
    } else {
        return Err("InvalidSignature");
    };
    let (keys, _siglen, _keylen) = match script.parse_redeem_script() {
        Some((k, s, l)) => (k, s, l),
        None => return Err("InvalidSignature"),
    };
    for sig in sigs.clone() {
        let mut verify = false;
        for key in keys.clone() {
            if verify_sign(&sig, &key, &tx, &script_pubkey) {
                verify = true;
                break;
            }
        }
        if !verify {
            return Err("Verify sign error");
        }
    }
    Ok(())
}
