use rstd::prelude::*;
use rstd::marker::PhantomData;
use rstd::result::Result as StdResult;

use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

use primitives::hash::{H160, H256};
use chain::Transaction;
use merkle::{PartialMerkleTree, parse_partial_merkle_tree};

use super::{
    Trait, ReceivePubkey, ReceivePubkeyHash,
    UTXOSet, TxSet, BlockTxids,
    BlockHeaderFor, HeaderNumberFor,
};

mod script;

#[derive(Clone, Encode, Decode)]
pub struct RelayTx {
    pub block_hash: H256,
    pub raw: Transaction,
    pub merkle_proof: PartialMerkleTree,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
pub struct UTXO {
    pub txid: H256,
    pub index: u32,
    pub balance: u64,
}

#[derive(Clone, Encode, Decode, Default)]
pub struct UTXOIndex(H256, u32);

struct UTXOStorage<T: Trait> (PhantomData<T>);

impl<T: Trait> UTXOStorage<T> {
    fn add(new_utxos: Vec<UTXO>) {
        for utxo in new_utxos {
            <UTXOSet<T>>::insert(UTXOIndex(utxo.txid.clone(), utxo.index), utxo.balance);
        }
    }

    fn remove(utxos_index: Vec<UTXOIndex>) {
        for u in utxos_index {
            <UTXOSet<T>>::remove(u)
        }
    }
}


pub trait RollBack<T: Trait> {
    fn rollback_tx(header: &H256) -> Result;
}

pub struct TxStorage<T: Trait> (PhantomData<T>);

impl<T: Trait> TxStorage<T> {
    fn store_tx(tx: &RelayTx, who: &T::AccountId) {
        let hash = tx.raw.hash();
        let block_hash = tx.block_hash.clone();
        let tx = tx.raw.clone();

        // todo 检查block是否存在
        <BlockTxids<T>>::mutate(block_hash, |v| v.push(hash.clone()));

        <TxSet<T>>::insert(hash, (tx, who.clone()));
    }

    fn check_previous(txid: &H256) -> bool {
        <TxSet<T>>::exists(txid)
    }
}

impl<T: Trait> RollBack<T> for TxStorage<T> {
    fn rollback_tx(header: &H256) -> Result {
        let receive_pubkeyhash = if let Some(h) = <ReceivePubkeyHash<T>>::get() { h } else {
            return Err("should set RECEIVE_PUBKEYHASH first");
        };

        let receive_pubkey = if let Some(h) = <ReceivePubkey<T>>::get() { h } else {
            return Err("should set RECEIVE_PUBKEY first");
        };

        let txids = <BlockTxids<T>>::get(header);
        for txid in txids.iter() {
            let (spent_utxos, new_utxos) = {
                let tx = <TxSet<T>>::get(txid).unwrap().0;
                let spent_utxos = handle_input(&tx, &receive_pubkey);
                let new_utxos = handle_output::<T>(&tx, &receive_pubkeyhash);
                (spent_utxos, new_utxos)
            };

            let mut rollback_spent: Vec<UTXO> = Vec::new();
            for (spent, _) in spent_utxos.iter() {
                let spent_tx = <TxSet<T>>::get(spent).unwrap().0;
                rollback_spent.append(&mut handle_output::<T>(&spent_tx, &receive_pubkeyhash));
            }

            let mut rollback_new: Vec<UTXOIndex> = new_utxos
                .iter()
                .map(|utxo| UTXOIndex(utxo.txid.clone(), utxo.index))
                .collect();

            <UTXOStorage<T>>::remove(rollback_new);
            <UTXOStorage<T>>::add(rollback_spent);
        }
        Ok(())
    }
}

pub fn validate_transaction<T: Trait>(tx: &RelayTx, who: &T::AccountId, receive_pubkeyhash: &[u8]) -> Result {
    if <HeaderNumberFor<T>>::exists(&tx.block_hash) == false {
        return Err("this tx's block not in the main chain");
    }

    let select_header = if let Some((header, _)) = <BlockHeaderFor<T>>::get(&tx.block_hash) {
        header
    } else {
        return Err("not has this block header yet");
    };

    let merkle_root = select_header.merkle_root_hash;
    let verify_txid = tx.raw.hash();
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

//    if tx.raw.inputs.iter().any(|input| {
//        input.script_sig.clone().take().ends_with(&receive_pubkeyhash) && (<TxStorage<T>>::check_previous(&input.previous_output.hash) == false)
//    }) {
//        return Err("previous tx not exist");
//    }
//
//    if tx.raw.inputs.iter().any(|input| {
//        input.script_sig.clone().take().ends_with(&receive_pubkeyhash)
//    }) {
//        <TxStorage<T>>::store_tx(tx, who);
//        return Ok(());
//    }
    let mut store_flag = false;
    for input in tx.raw.inputs.iter() {
        if input.script_sig.as_ref().ends_with(&receive_pubkeyhash) {
            if <TxStorage<T>>::check_previous(&input.previous_output.hash) == false {
                return Err("previous tx not exist yet for this input");
            }
            store_flag = true;
        }
    }
    if store_flag {
        <TxStorage<T>>::store_tx(tx, who);
        return Ok(());
    }

    for output in tx.raw.outputs.iter() {
        let script = &output.script_pubkey;
        match script::parse_script(script) {
            script::ParseScript::PubKeyHash => {
                if receive_pubkeyhash == script.as_ref() {
                    <TxStorage<T>>::store_tx(tx, who);
                    return Ok(());
                }
            }
            _ => {
                continue;
            }
        }
    }

    Err("not found our pubkey, may be an unrelated tx")
}

pub fn handle_input(tx: &Transaction, receive_pubkeyhash: &[u8]) -> Vec<(H256, u32)> {
    // remove utxos.
    tx.inputs
        .iter()
        .filter(|input| {
            input.script_sig.as_ref().ends_with(
                receive_pubkeyhash
            )
        })
        .map(|input| {
            (
                input.previous_output.hash.clone(),
                input.previous_output.index,
            )
        })
        .collect()
}

pub fn deposit_token(tx: &Transaction, output_index: usize) -> StdResult<(H160, u64), ()> {
    // parse input pubkey hash, only handle first input
    if let Ok(pubkey_hash) = script::parse_sigscript(&tx.inputs[0].script_sig) {
        let balance = tx.outputs[output_index].value;
        return Ok((pubkey_hash, balance));
    }
    return Err(());
}

pub fn handle_output<T: Trait>(tx: &Transaction, receive_pubkeyhash: &[u8]) -> Vec<UTXO> {
    let mut new_utxos = Vec::<UTXO>::new();
    // Add utxo
    for (index, output) in tx.outputs.iter().enumerate() {
        let script = &output.script_pubkey;
        match script::parse_script(script) {
            script::ParseScript::PubKeyHash => {
                if receive_pubkeyhash == script.as_slice() {
                    if let Ok((pubkey_hash, balance)) = deposit_token(tx, index) {
                        //TODO: depoist
                    }
                    new_utxos.push(UTXO {
                        txid: tx.hash(),
                        index: index as u32,
                        balance: output.value,
                    })
                }
            }
            _ => {
                continue;
            }
        }
    }
    new_utxos
}
