// Copyright 2018 Chainpool

use codec::Decode;
use rstd::marker::PhantomData;
use rstd::prelude::*;
use rstd::result::Result as StdResult;

pub use self::proposal::{handle_proposal, Proposal};
use super::{
    AddressMap, BTCTxLog, BindInfo, BlockHeaderFor, BlockTxsMapKeys, CandidateTx, CertCache,
    DepositCache, DepositHistInfo, DepositInfo, DepositRecordsMap, LinkedNodes, Module, NetworkId,
    Node, NumberForHash, Trait, TrusteeAddress, TxProposal, TxSet, TxSetTail, TxType, UTXOSet,
    UTXOSetLen,
};
use b58::from;
use chain::{OutPoint, Transaction, TransactionInput, TransactionOutput};
use keys;
use merkle::{parse_partial_merkle_tree, PartialMerkleTree};
use primitives::bytes::Bytes;
use primitives::hash::H256;
use runtime_io;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};
use script::script::Script;
use script::{
    builder, SignatureChecker, SignatureVersion, TransactionInputSigner,
    TransactionSignatureChecker,
};
use system;
use utils::vec_to_u32;
mod proposal;

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
    // To do: Optmise select OutPoint algorithm
    fn select_utxo(balance: u64) -> Option<Vec<UTXO>> {
        let mut index = <UTXOSetLen<T>>::get();
        let mut count_balance = 0;
        let mut utxo_set = Vec::new();
        while index > 0 {
            index -= 1;
            let utxo = <UTXOSet<T>>::get(index);
            if utxo.is_spent == false && utxo.balance > 0 {
                utxo_set.push(utxo.clone());
                count_balance += utxo.balance;
                if count_balance >= balance {
                    return Some(utxo_set);
                }
            }
        }
        None
    }

    fn add(new_utxos: Vec<UTXO>) {
        let mut index = <UTXOSetLen<T>>::get();
        for utxo in new_utxos {
            <UTXOSet<T>>::insert(index, utxo.clone());
            index += 1;
        }
        <UTXOSetLen<T>>::mutate(|inc| *inc = index);
    }

    fn update(utxos: Vec<UTXO>, is_spent: bool) -> u64 {
        let mut update_balance = 0;
        for u in utxos {
            let mut index = <UTXOSetLen<T>>::get();
            while index > 0 {
                index -= 1;
                let utxo = <UTXOSet<T>>::get(index);
                if utxo == u {
                    <UTXOSet<T>>::mutate(index, |utxo| {
                        update_balance += utxo.balance;
                        utxo.is_spent = is_spent;
                    });
                    break;
                }
            }
        }
        update_balance
    }

    fn update_from_outpoint(out_point_set: Vec<OutPoint>, is_spent: bool) -> u64 {
        let mut update_balance = 0;
        for out_point in out_point_set {
            let mut index = <UTXOSetLen<T>>::get();
            while index > 0 {
                index -= 1;
                let utxo = <UTXOSet<T>>::get(index);
                if out_point.hash == utxo.txid && out_point.index == utxo.index {
                    <UTXOSet<T>>::mutate(index, |utxo| {
                        update_balance += utxo.balance;
                        utxo.is_spent = is_spent;
                    });
                    break;
                }
            }
        }
        update_balance
    }

    #[allow(unused)]
    fn find_utxo(out_point: &OutPoint) -> Option<UTXO> {
        let mut index = <UTXOSetLen<T>>::get();
        while index > 0 {
            index -= 1;
            let utxo = <UTXOSet<T>>::get(index);
            if out_point.hash == utxo.txid && out_point.index == utxo.index {
                return Some(utxo);
            }
        }
        None
    }
}

pub trait RollBack<T: Trait> {
    fn rollback_tx(header: &H256) -> Result;
}

pub struct TxStorage<T: Trait>(PhantomData<T>);

impl<T: Trait> TxStorage<T> {
    fn store_tx(tx: &Transaction, block_hash: &H256, tx_type: TxType) {
        let hash = tx.hash();
        let block_hash = block_hash.clone();
        let tx = tx.clone();

        // todo 检查block是否存在
        <BlockTxsMapKeys<T>>::mutate(block_hash.clone(), |v| v.push(hash.clone()));

        let log = BTCTxLog { tx_type, tx };
        let n = Node::new(log);
        // insert to the storage, same to TxSet::<T>::insert(hash, xxx)
        n.init_storage::<LinkedNodes<T>>();
        if let Some(tail_index) = TxSetTail::<T>::get() {
            // get tail
            if let Some(mut tail) = TxSet::<T>::get(tail_index.index()) {
                // add tx to tail
                let _ = tail.add_option_node_after::<LinkedNodes<T>>(n);
            }
        }
    }

    #[allow(unused)]
    fn find_tx(txid: &H256) -> Option<Transaction> {
        if let Some(btc_tx_log) = <TxSet<T>>::get(txid) {
            return Some(btc_tx_log.data.tx);
        } else {
            return None;
        }
    }
}

impl<T: Trait> RollBack<T> for TxStorage<T> {
    fn rollback_tx(header: &H256) -> Result {
        let trustee_address: keys::Address = if let Some(h) = <TrusteeAddress<T>>::get() {
            h
        } else {
            return Err("should set RECEIVE_ADDRESS first");
        };

        let txids = <BlockTxsMapKeys<T>>::get(header);
        for txid in txids.iter() {
            let btc_tx_log = <TxSet<T>>::get(txid).unwrap();
            let data = btc_tx_log.data;
            match data.tx_type {
                TxType::Withdraw => {
                    let out_point_set = data
                        .tx
                        .inputs
                        .iter()
                        .map(|input| input.previous_output.clone())
                        .collect();
                    <UTXOStorage<T>>::update_from_outpoint(out_point_set, false);
                    let mut out_point_set2: Vec<OutPoint> = Vec::new();
                    let mut index = 0;

                    let _ = data
                        .tx
                        .outputs
                        .iter()
                        .map(|output| {
                            if is_key(&output.script_pubkey, &trustee_address) {
                                out_point_set2.push(OutPoint {
                                    hash: txid.clone(),
                                    index: index,
                                })
                            }
                            index += 1;
                            ()
                        })
                        .collect::<()>();
                    <UTXOStorage<T>>::update_from_outpoint(out_point_set2, true);
                }
                TxType::Register => {}
                _ => {
                    let mut rollback_spent: Vec<UTXO> = Vec::new();
                    for (index, output) in data.tx.outputs.iter().enumerate() {
                        rollback_spent.push(UTXO {
                            txid: data.tx.hash(),
                            index: index as u32,
                            balance: output.value,
                            is_spent: false,
                        });
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
    address: (&keys::Address, &keys::Address),
) -> StdResult<TxType, &'static str> {
    if <TxSet<T>>::exists(&tx.raw.hash()) == true {
        return Err("this tx already store");
    }

    if <NumberForHash<T>>::exists(&tx.block_hash) == false {
        return Err("this tx's block not in the main chain");
    }

    let select_header = if let Some((header, _, _)) = <BlockHeaderFor<T>>::get(&tx.block_hash) {
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

    // To do: All inputs relay
    let previous_txid = tx.previous_raw.hash();
    if previous_txid != tx.raw.inputs[0].previous_output.hash {
        return Err("previous tx id not right");
    }

    // detect cert
    let outpoint = tx.raw.inputs[0].previous_output.clone();
    let send_address = inspect_address::<T>(&tx.previous_raw, outpoint).unwrap();
    if send_address.hash == address.1.hash {
        return Ok(TxType::SendCert);
    }

    // detect withdraw: To do: All inputs relay
    let outpoint = tx.raw.inputs[0].previous_output.clone();
    let send_address = inspect_address::<T>(&tx.previous_raw, outpoint).unwrap();
    if send_address.hash == address.0.hash {
        return Ok(TxType::Withdraw);
    }

    // detect deposit
    for output in tx.raw.outputs.iter() {
        if is_key(&output.script_pubkey, &address.0) {
            return Ok(TxType::RegisterDeposit);
        }
    }

    Err("not found our pubkey, may be an unrelated tx")
}

fn is_key(script_pubkey: &[u8], trustee_address: &keys::Address) -> bool {
    let script: Vec<u8> = script_pubkey.iter().cloned().collect();
    let script: Script = script.into();
    let script_addresses = script.extract_destinations().unwrap_or(Vec::new());
    if script_addresses.len() == 1 {
        if trustee_address.hash == script_addresses[0].hash {
            return true;
        }
    }
    return false;
}

fn compare_transaction(tx1: &Transaction, tx2: &Transaction) -> bool {
    if tx1.version == tx2.version && tx1.outputs == tx2.outputs && tx1.lock_time == tx2.lock_time {
        if tx1.inputs.len() == tx2.inputs.len() {
            for i in 0..tx1.inputs.len() {
                if tx1.inputs[i].previous_output == tx2.inputs[i].previous_output
                    && tx1.inputs[i].sequence == tx2.inputs[i].sequence
                {
                    return true;
                }
            }
        }
        return false;
    }
    return false;
}

pub fn handle_input<T: Trait>(
    tx: &Transaction,
    block_hash: &H256,
    who: &T::AccountId,
    trustee_address: &keys::Address,
) {
    let len = Module::<T>::tx_proposal_len();
    if len > 0 {
        let tx2 = <TxProposal<T>>::get(len - 1);
        if tx2.is_some() {
            let mut candidate = tx2.clone().unwrap();
            if candidate.confirmed == false {
                // 当提上来的提现交易和待签名原文不一致时， 说明系统BTC托管有异常，标记unexpect
                if compare_transaction(tx, &tx2.unwrap().tx) {
                    candidate.block_hash = block_hash.clone();
                } else {
                    candidate.unexpect = true;
                }
                <TxProposal<T>>::insert(len - 1, candidate);
            }
        } else {
            // Todo: handle_input error not expect
            runtime_io::print("-----------handle_input error not expect");
        }
    }
    let out_point_set = tx
        .inputs
        .iter()
        .map(|input| input.previous_output.clone())
        .collect();
    let mut update_balance = <UTXOStorage<T>>::update_from_outpoint(out_point_set, true);
    runtime_io::print("-----------handle_input update_balance");
    runtime_io::print(update_balance);

    let time = <system::Module<T>>::block_number();
    let mut new_utxo: Vec<UTXO> = Vec::new();
    let mut index = 0;
    let _ = tx
        .outputs
        .iter()
        .map(|output| {
            if is_key(&output.script_pubkey, &trustee_address) {
                update_balance -= output.value;
                new_utxo.push(UTXO {
                    txid: tx.hash(),
                    index: index,
                    balance: output.value,
                    is_spent: false,
                });
            }
            index += 1;
            ()
        })
        .collect::<()>();
    <UTXOStorage<T>>::add(new_utxo);
    <TxStorage<T>>::store_tx(tx, block_hash, TxType::Withdraw);
}

fn deposit_token<T: Trait>(
    address: &keys::Address,
    balance: u64,
    txid: &H256,
    block_hash: &H256,
    channel: Vec<u8>,
) -> bool {
    let bind_info = match <AddressMap<T>>::get(address) {
        Some(info) => info,
        None => {
            return false;
        }
    };

    let mut deposit: Vec<DepositInfo<T::AccountId>> = match <DepositCache<T>>::take() {
        Some(deposit) => deposit,
        None => Vec::new(),
    };
    let cache: DepositInfo<T::AccountId> = DepositInfo {
        account: bind_info.account,
        btc_balance: balance,
        tx_hash: txid.clone(),
        block_hash: block_hash.clone(),
        channel: channel,
    };
    deposit.push(cache);
    <DepositCache<T>>::put(deposit);
    return true;
}

fn inspect_address<T: Trait>(tx: &Transaction, outpoint: OutPoint) -> Option<keys::Address> {
    let script: Script = tx.outputs[outpoint.index as usize]
        .script_pubkey
        .clone()
        .into();
    let script_addresses = script.extract_destinations().unwrap_or(Vec::new());
    if script_addresses.len() == 1 {
        let address = &script_addresses[0];
        let network_id = <NetworkId<T>>::get();
        let network = if network_id == 1 {
            keys::Network::Testnet
        } else {
            keys::Network::Mainnet
        };
        let address = keys::Address {
            kind: address.kind,
            network: network,
            hash: address.hash.clone(),
        };
        return Some(address);
    }
    return None;
}

pub fn handle_output<T: Trait>(
    tx: &Transaction,
    block_hash: &H256,
    previous_tx: &Transaction,
    trustee_address: &keys::Address,
) {
    let mut new_utxos = Vec::<UTXO>::new();
    let mut total_balance: u64 = 0;
    let mut register = false;
    let tx_type;
    let mut deposit_channel = Vec::<u8>::new();
    // Add utxo
    let tx_hash = tx.hash();
    let outpoint = tx.inputs[0].previous_output.clone();
    let send_address = inspect_address::<T>(previous_tx, outpoint).unwrap();
    for (index, output) in tx.outputs.iter().enumerate() {
        let script = &output.script_pubkey;
        let script: Script = script.clone().into();
        // bind address [btc address --> chainx AccountId]
        if script.is_null_data_script() {
            let data = script.extract_rear(':');
            let slice: Vec<u8> = match from(data) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let slice = slice.as_slice();
            if slice.len() < 32 {
                continue;
            }
            let mut account: Vec<u8> = Vec::new();
            account.extend_from_slice(&slice[1..33]);
            runtime_io::print(&account[..]);
            let id: T::AccountId = match Decode::decode(&mut account.as_slice()) {
                Some(a) => a,
                None => continue,
            };
            deposit_channel = script.extract_pre(':');
            deposit_channel = deposit_channel[2..].to_vec();
            let bind_info: BindInfo<T::AccountId> = BindInfo {
                account: id,
                channel: deposit_channel.clone(),
            };
            <AddressMap<T>>::insert(&send_address, bind_info);
            register = true;
            runtime_io::print("=================== handle_output register ");
            match <DepositRecordsMap<T>>::get(&send_address) {
                Some(records) => {
                    for rec in records.iter() {
                        if deposit_token::<T>(
                            &send_address,
                            rec.btc_balance,
                            &rec.tx_hash,
                            &rec.block_hash,
                            deposit_channel.clone(),
                        ) {
                            runtime_io::print("=================== handle_output DepositRecordsMap ");
                            runtime_io::print(rec.btc_balance);
                        }
                    }
                    // 把历史充值记录放入 DepositCache 后，直接删除历史记录
                    <DepositRecordsMap<T>>::remove(&send_address.clone());
                }
                None => {}
            };
            continue;
        }
        // get deposit money
        let script_addresses = script.extract_destinations().unwrap_or(Vec::new());
        if script_addresses.len() == 1 {
            if (trustee_address.hash == script_addresses[0].hash) && (output.value > 0) {
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
    runtime_io::print("=================== handle_output deposit_token: ");
    runtime_io::print(total_balance);
    if deposit_token::<T>(
        &send_address,
        total_balance,
        &tx_hash,
        block_hash,
        deposit_channel.clone(),
    ) {
        runtime_io::print(total_balance);
    } else {
        let mut hist: Vec<DepositHistInfo> = match <DepositRecordsMap<T>>::get(&send_address) {
            Some(vec) => vec,
            None => Vec::new(),
        };
        // 根据渠道名称获取渠道ID,填入历史充值记录
        let cache = DepositHistInfo {
            btc_balance: total_balance,
            tx_hash: tx_hash.clone(),
            block_hash: block_hash.clone(),
            channel: deposit_channel,
        };
        hist.push(cache);
        <DepositRecordsMap<T>>::insert(&send_address, hist);
    }

    let time = <system::Module<T>>::block_number();
    if register {
        if total_balance == 0 {
            tx_type = TxType::Register;
        } else {
            tx_type = TxType::RegisterDeposit;
        }
    } else {
        tx_type = TxType::Deposit;
    };
    <TxStorage<T>>::store_tx(tx, block_hash, tx_type);
    <UTXOStorage<T>>::add(new_utxos.clone());
}

pub fn handle_cert<T: Trait>(tx: &Transaction) {
    for (_index, output) in tx.outputs.iter().enumerate() {
        let script = &output.script_pubkey;
        let script: Script = script.clone().into();
        if script.is_null_data_script() {
            let pre = script.extract_pre(':');
            let rear = script.extract_rear(':');
            let mut date = Vec::new();
            let mut account = Vec::new();
            let mut name = Vec::new();
            let mut current = 0;
            while current < rear.len() {
                if rear[current] == ':' as u8 {
                    break;
                }
                current += 1;
            }
            date.extend_from_slice(&rear[0..current]);
            account.extend_from_slice(&rear[current + 1..]);
            name.extend_from_slice(&pre[2..]);
            let frozen_duration = if let Some(date) = vec_to_u32(date.clone()) {
                date
            } else {
                0
            };
            let slice = from(account).unwrap();
            let slice = slice.as_slice();
            let mut account: Vec<u8> = Vec::new();
            account.extend_from_slice(&slice[1..33]);
            runtime_io::print(&account[..]);
            let id: T::AccountId = Decode::decode(&mut account.as_slice()).unwrap();
            let mut cache: Vec<(Vec<u8>, u32, T::AccountId)> = Vec::new();
            cache.push((name, frozen_duration, id));
            <CertCache<T>>::put(cache);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_id() {
        let script = Script::from(
            "chainx:5HnDcuKFCvsR42s8Tz2j2zLHLZAaiHG4VNyJDa7iLRunRuhM"
                .as_bytes()
                .to_vec(),
        );
        let data = script.extract_rear(':');
        let slice = from(data).unwrap();
        let slice = slice.as_slice();
        let mut account: Vec<u8> = Vec::new();
        account.extend_from_slice(&slice[1..33]);
        assert_eq!(account.len(), 32);
        let id: H256 = Decode::decode(&mut account.as_slice()).unwrap();
        assert_eq!(
            id,
            H256::from("fcd66b3b5a737f8284fef82d377d9c2391628bbe11ec63eb372b032ce2618725")
        );
    }

    #[test]
    fn test_cert() {
        let script = Script::from(
            "chainx:66990:5HnDcuKFCvsR42s8Tz2j2zLHLZAaiHG4VNyJDa7iLRunRuhM"
                .as_bytes()
                .to_vec(),
        );

        let rear = script.extract_rear(':');
        let mut date = Vec::new();
        let mut owner = Vec::new();
        let mut current = 0;
        while current < rear.len() {
            if rear[current] == ':' as u8 {
                break;
            }
            current += 1;
        }
        date.extend_from_slice(&rear[0..current]);
        owner.extend_from_slice(&rear[current + 1..]);

        let frozen_duration = if let Some(date) = vec_to_u32(date) {
            date
        } else {
            0
        };
        assert_eq!(66990, frozen_duration);
    }

    #[test]
    fn test_cert() {
        let script = Script::from(
            "chainx:66990:5HnDcuKFCvsR42s8Tz2j2zLHLZAaiHG4VNyJDa7iLRunRuhM"
                .as_bytes()
                .to_vec(),
        );

        let name = script.extract_pre(':');
        let rear = script.extract_rear(':');
        let mut date = Vec::new();
        let mut owner = Vec::new();
        let mut current = 0;
        while current < rear.len() {
            if rear[current] == ':' as u8 {
                break;
            }
            current += 1;
        }
        date.extend_from_slice(&rear[0..current]);
        owner.extend_from_slice(&rear[current + 1..]);

        let frozen_duration = if let Some(date) = vec_to_u32(date) {
            date
        } else {
            0
        };
        assert_eq!(66990, frozen_duration);
    }
}
