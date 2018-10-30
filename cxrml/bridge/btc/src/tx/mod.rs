// Copyright 2018 Chainpool

use rstd::prelude::*;
use rstd::marker::PhantomData;
use rstd::result::Result as StdResult;
use codec::Decode;

use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

use primitives::hash::{H160, H256};
use chain::{Transaction, OutPoint};
use merkle::{PartialMerkleTree, parse_partial_merkle_tree};
use runtime_io;
use script::script::{Script, ScriptAddress};
use keys;
use keys::DisplayLayout;

use super::{TxType, Trait, ReceiveAddress, AddressMap, UTXOSet,
            TxSet, BlockTxids, BlockHeaderFor, HeaderNumberFor, UTXOMaxIndex,};

#[derive(Clone, Encode, Decode)]
pub struct RelayTx {
    pub block_hash: H256,
    pub raw: Transaction,
    pub merkle_proof: PartialMerkleTree,
    pub previous_raw: Transaction,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
pub struct UTXO {
    pub txid: H256,
    pub index: u32,
    pub balance: u64,
    pub is_spent: bool,
}

struct UTXOStorage<T: Trait>(PhantomData<T>);

impl<T: Trait> UTXOStorage<T> {
    fn add(new_utxos: Vec<UTXO>) {
        let mut index = <UTXOMaxIndex<T>>::get();
        for utxo in new_utxos {
            <UTXOSet<T>>::insert(index, utxo.clone());
            index += 1;
        }
        <UTXOMaxIndex<T>>::mutate(|inc| *inc = index);
    }

    fn update(utxos: Vec<UTXO>, is_spent: bool) {
        for u in utxos {
            let mut index = <UTXOMaxIndex<T>>::get() - 1;
            while index >= 0 {
               let utxo = <UTXOSet<T>>::get(index);
               if utxo == u {
                   <UTXOSet<T>>::mutate(index, |utxo| utxo.is_spent = is_spent);
                   break;
               }
               index -= 1;
            }
        }
    }

    fn update_from_outpoint(out_point_set: Vec<OutPoint>, is_spent: bool) {
        for out_point in out_point_set {
            let mut index = <UTXOMaxIndex<T>>::get() - 1;
            while index >= 0 {
                let utxo = <UTXOSet<T>>::get(index);
                if out_point.hash == utxo.txid && out_point.index == utxo.index {
                    <UTXOSet<T>>::mutate(index, |utxo| utxo.is_spent = is_spent );
                    break;
                }
                index -= 1;
            }
        }
    }
}


pub trait RollBack<T: Trait> {
    fn rollback_tx(header: &H256) -> Result;
}

pub struct TxStorage<T: Trait>(PhantomData<T>);

impl<T: Trait> TxStorage<T> {
    fn store_tx(tx: &Transaction, block_hash: &H256,  who: &T::AccountId, address: keys::Address, tx_type: TxType, balance: u64) {
        let hash = tx.hash();
        let block_hash = block_hash.clone();
        let tx = tx.clone();

        // todo 检查block是否存在
        <BlockTxids<T>>::mutate(block_hash, |v| v.push(hash.clone()));

        <TxSet<T>>::insert(hash, (tx, who.clone(), address, tx_type, balance));
    }

    fn check_previous(txid: &H256) -> bool {
        <TxSet<T>>::exists(txid)
    }
}

impl<T: Trait> RollBack<T> for TxStorage<T> {
    fn rollback_tx(header: &H256) -> Result {
        let receive_address = if let Some(h) = <ReceiveAddress<T>>::get() {
            h
        } else {
            return Err("should set RECEIVE_ADDRESS first");
        };

        let txids = <BlockTxids<T>>::get(header);
        for txid in txids.iter() {
            let (tx, _, _, tx_type, _) = <TxSet<T>>::get(txid).unwrap();
            match tx_type {
                TxType::Withdraw => {
                         let out_point_set = tx.inputs.iter().map(|input| input.previous_output.clone()).collect();
                         <UTXOStorage<T>>::update_from_outpoint(out_point_set, false);
                     },
                TxType::Register => {},
                _ => {
                    let mut rollback_spent: Vec<UTXO> = Vec::new();
                    for (index, output) in tx.outputs.iter().enumerate() {
                        rollback_spent.push(UTXO {  txid: tx.hash(),
                                                    index: index as u32,
                                                    balance: output.value, is_spent: false});
                    }
                    <UTXOStorage<T>>::update(rollback_spent, true);
                }
            }
        }
        Ok(())
    }
}

pub fn validate_transaction<T: Trait>(
    tx: &RelayTx,
    receive_address: &[u8],
) -> StdResult<TxType, &'static str> {
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
    let previous_txid = tx.previous_raw.hash();
    if previous_txid != tx.raw.inputs[0].previous_output.hash {
        return Err("previous tx id not right");
    }
    let receive_address = keys::Address::from_layout(receive_address).unwrap();
    // detect withdraw
    for input in tx.raw.inputs.iter() {
        let outpoint = input.previous_output.clone();
        let send_address = inspect_address(&tx.previous_raw, outpoint).unwrap();        
        if send_address.hash == receive_address.hash {
            return Ok(TxType::Withdraw);
        }
    }
    // detect deposit
    for output in tx.raw.outputs.iter() {
        let script = output.script_pubkey.clone().take();
        runtime_io::print("------script");
        runtime_io::print(script.as_slice());
        let script: Script = script.clone().into();
        let script_addresses = script.extract_destinations().unwrap_or(vec![]);
        if script_addresses.len() == 1 {
            if receive_address.hash == script_addresses[0].hash {
                return Ok(TxType::RegisterDeposit);
            }
        }
    }

    Err("not found our pubkey, may be an unrelated tx")
}

pub fn handle_input<T: Trait>(tx: &Transaction, block_hash: &H256, who: &T::AccountId, receive_address: &[u8]){
    let out_point_set = tx.inputs.iter().map(|input| {
            input.previous_output.clone() }).collect();
    <UTXOStorage<T>>::update_from_outpoint(out_point_set, true);
    let receive_address = keys::Address::from_layout(receive_address).unwrap();
    let mut total_balance = 0;
    tx.outputs.iter().map(|output| { total_balance += output.value; ()});
    <TxStorage<T>>::store_tx(tx, block_hash, who, receive_address.clone(), TxType::Withdraw, total_balance);
}

fn deposit_token<T: Trait>(_address: keys::Address, _balance: u64) {
    // to do: 
}

fn inspect_address(tx: &Transaction, outpoint: OutPoint) -> Option<keys::Address> {
    let script: Script = tx.outputs[outpoint.index as usize].script_pubkey.clone().into();
    let script_addresses = script.extract_destinations().unwrap_or(vec![]);
    if script_addresses.len() == 1 {
        let address = &script_addresses[0];
        return Some(address.into());
    }
    return None;
}

pub fn handle_output<T: Trait>(tx: &Transaction, block_hash: &H256, who: &T::AccountId, previous_tx: &Transaction, receive_address: &[u8]) -> Vec<UTXO> {
    let mut new_utxos = Vec::<UTXO>::new();
    let mut total_balance: u64 = 0;
    let mut register = false;
    // Add utxo
    let receive_address = keys::Address::from_layout(receive_address).unwrap();
    let outpoint = tx.inputs[0].previous_output.clone();
    let send_address = inspect_address(previous_tx, outpoint).unwrap();
    for (index, output) in tx.outputs.iter().enumerate() {
        let script = &output.script_pubkey;
        let script: Script = script.clone().into();
        // bind address [btc address --> chainx AccountId]
        if script.is_null_data_script() {
            let data = script.extract_rear(':');
            runtime_io::print("------opreturn account data-----");
            runtime_io::print(&data[..]);
            let id: T::AccountId = Decode::decode(&mut data.as_slice()).unwrap();
            <AddressMap<T>>::insert(send_address.clone(), id);
            register = true;
            continue;
        }
        // deposit money
        let script_addresses = script.extract_destinations().unwrap_or(vec![]);
        if script_addresses.len() == 1 {
            if receive_address.hash == script_addresses[0].hash {
                runtime_io::print("------deposit_token-----");
                deposit_token::<T>(send_address.clone(), output.value);
                total_balance += output.value;
                new_utxos.push(UTXO {
                    txid: tx.hash(),
                    index: index as u32,
                    balance: output.value,
                    is_spent: false,
                });
            }
        }
    }
    if total_balance == 0 {
        <TxStorage<T>>::store_tx(tx, block_hash, who, send_address.clone(), TxType::Register, total_balance);
    } else {
        if register {
            <TxStorage<T>>::store_tx(tx, block_hash, who, send_address.clone(), TxType::RegisterDeposit, total_balance);
        } else {
            <TxStorage<T>>::store_tx(tx, block_hash, who, send_address.clone(), TxType::Deposit, total_balance);
        } 
        <UTXOStorage<T>>::add(new_utxos.clone());
    }
    runtime_io::print("------store tx success-----");
    new_utxos
}
