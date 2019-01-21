use super::*;

pub fn validate_transaction<T: Trait>(
    tx: &RelayTx,
    address: (&keys::Address, &keys::Address),
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

    // detect cert
    let outpoint = tx.raw.inputs[0].previous_output.clone();
    let send_address = match inspect_address::<T>(&tx.previous_raw, outpoint) {
        Some(a) => a,
        None => return Err("inspect address failed at detect cert-tx "),
    };
    if send_address.hash == address.1.hash {
        return Ok(TxType::SendCert);
    }

    // detect withdraw: To do: All inputs relay
    let outpoint = tx.raw.inputs[0].previous_output.clone();
    let send_address = match inspect_address::<T>(&tx.previous_raw, outpoint) {
        Some(a) => a,
        None => return Err("inspect address failed at detect withdraw-tx "),
    };
    if send_address.hash == address.0.hash {
        return Ok(TxType::Withdraw);
    }

    // detect deposit
    for output in tx.raw.outputs.iter() {
        if is_key(&output.script_pubkey, &address.0) {
            return Ok(TxType::BindDeposit);
        }
    }

    Err("not found our pubkey, may be an unrelated tx")
}
