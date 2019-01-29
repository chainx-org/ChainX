use self::extracter::Extracter;
use super::*;
use b58;
use keys::DisplayLayout;

pub struct TxHandler<'a>(&'a H256);

impl<'a> TxHandler<'a> {
    pub fn new(txid: &H256) -> TxHandler {
        TxHandler(txid)
    }

    pub fn cert<T: Trait>(&self) -> Result {
        runtime_io::print("[bridge-btc] handle_cert start");

        let txid = self.0;
        let tx_info = <TxFor<T>>::get(txid);
        for (_index, output) in tx_info.raw_tx.outputs.iter().enumerate() {
            let script = &output.script_pubkey;
            let into_script: Script = script.clone().into();

            if into_script.is_null_data_script() {
                let s = script.clone();
                let (cert_name, frozen_duration, cert_owner) = Extracter::<T>::new(&s)
                    .cert()
                    .ok_or("Fail to parse OP_RETURN.")?;

                runtime_io::print("[bridge-btc] issue cert");
                Module::<T>::deposit_event(RawEvent::CertTx(
                    tx_info.raw_tx.hash(),
                    tx_info.input_address.layout().to_vec(),
                ));
            }
        }

        Ok(())
    }

    pub fn withdraw<T: Trait>(&self, trustee_address: &keys::Address) -> Result {
        runtime_io::print("[bridge-btc] handle_withdraw start");

        //delete used
        let txid = self.0;
        let tx_info = <TxFor<T>>::get(txid);
        let out_point_set = tx_info
            .raw_tx
            .inputs
            .iter()
            .map(|input| input.previous_output.clone())
            .collect();
        delete_utxo::<T>(out_point_set);
        runtime_io::print("[bridge-btc] handle_input delete_from_outpoint");

        //Give change
        let mut index = 0;
        for output in tx_info.raw_tx.clone().outputs {
            if is_key(&output.script_pubkey, &trustee_address) {
                refresh_utxo::<T>(
                    UTXOKey {
                        txid: txid.clone(),
                        index: index as u32,
                    },
                    UTXOStatus {
                        balance: output.value,
                        status: true,
                    },
                );
            }
            index += 1;
        }
        runtime_io::print("[bridge-btc] handle_input refresh_utxo");
        let mut flag = false;
        if let Some(data) = <TxProposal<T>>::take() {
            let candidate = data.clone();
            match ensure_identical(&tx_info.raw_tx, &data.tx) {
                Ok(()) => {
                    flag = true;
                    let txid = candidate.tx.hash();
                    for number in candidate.outs.iter() {
                        runtime_io::print(*number as u64);
                        runtime_io::print(&txid[..]);

                        <xrecords::Module<T>>::withdrawal_finish(*number, true)?;
                        runtime_io::print("[bridge-btc] withdrawal finish");
                    }
                }
                Err(_) => {
                    <TxProposal<T>>::put(data);
                    runtime_io::print("[bridge-btc] withdrawal failed");
                }
            };
        }
        let addr = tx_info.input_address.layout().to_vec();
        Module::<T>::deposit_event(RawEvent::WithdrawTx(
            tx_info.raw_tx.hash(),
            b58::to_base58(addr),
            flag,
        ));
        Ok(())
    }

    pub fn deposit<T: Trait>(&self, trustee_address: &keys::Address) {
        runtime_io::print("[bridge-btc] handle_output start");

        let txid = self.0;
        let tx_info = <TxFor<T>>::get(txid);
        // Add utxo
        for (index, output) in tx_info.raw_tx.outputs.iter().enumerate() {
            let script = &output.script_pubkey;
            let into_script: Script = script.clone().into();

            // bind address [btc address --> chainx AccountId]
            if into_script.is_null_data_script() {
                let s = script.clone();
                handle_opreturn::<T>(&s, &tx_info);
                continue;
            }

            // get deposit money
            // FIXME should detect if the script_addresses exists in a better way.
            let script_addresses = into_script.extract_destinations().unwrap_or(Vec::new());
            if script_addresses.len() == 1 {
                if (trustee_address.hash == script_addresses[0].hash) && (output.value > 0) {
                    let mut deposit_status = false;
                    let input_address = &tx_info.input_address;

                    <AddressMap<T>>::get(input_address).map_or_else(
                        || insert_pending_deposit::<T>(input_address, txid.clone(), index as u32),
                        |account| {
                            deposit_token::<T>(&account, output.value);
                            runtime_io::print("[bridge-btc] handle_output deposit_token: ");
                            deposit_status = true;
                        },
                    );

                    runtime_io::print(output.value);

                    refresh_utxo::<T>(
                        UTXOKey {
                            txid: txid.clone(),
                            index: index as u32,
                        },
                        UTXOStatus {
                            balance: output.value,
                            status: deposit_status,
                        },
                    );
                    let addr = tx_info.input_address.layout().to_vec();
                    Module::<T>::deposit_event(RawEvent::Deposit(
                        tx_info.raw_tx.hash(),
                        b58::to_base58(addr),
                        output.value,
                        deposit_status,
                    ));
                }
            }
        }
    }
}

fn delete_utxo<T: Trait>(out_point_set: Vec<OutPoint>) -> bool {
    if let Some(mut keys) = <UTXOSetKey<T>>::take() {
        for out_point in out_point_set {
            let mut count = 0;
            for (i, k) in keys.iter().enumerate() {
                if out_point.hash == k.txid && out_point.index == k.index {
                    <UTXOSet<T>>::remove(k);
                    count = i;
                    break;
                }
            }
            keys.remove(count);
        }

        <UTXOSetKey<T>>::put(keys);

        return true;
    }

    false
}

/// Try updating the binding address, remove pending deposit if the updating goes well.
fn handle_opreturn<T: Trait>(script: &[u8], info: &TxInfo) {
    if let Some(account_id) = Extracter::<T>::new(&script).account_id() {
        if update_binding::<T>(&account_id.clone(), info) {
            runtime_io::print("[bridge-btc] handle_output register ");
            remove_pending_deposit::<T>(&info.input_address, &account_id);
        }
    }
}

fn ensure_identical(tx1: &Transaction, tx2: &Transaction) -> Result {
    if tx1.version == tx2.version
        && tx1.outputs == tx2.outputs
        && tx1.lock_time == tx2.lock_time
        && tx1.inputs.len() == tx2.inputs.len()
    {
        for i in 0..tx1.inputs.len() {
            if tx1.inputs[i].previous_output != tx2.inputs[i].previous_output
                || tx1.inputs[i].sequence != tx2.inputs[i].sequence
            {
                return Err("The inputs of these two transactions mismatch.");
            }
        }
        return Ok(());
    }
    Err("The transaction text does not match the original text to be signed.")
}

/// Actually update the binding address of original transactor.
fn apply_update_binding<T: Trait>(who: &T::AccountId, address: &keys::Address) {
    match <AccountMap<T>>::get(who) {
        Some(mut a) => {
            a.push(address.clone());
            <AccountMap<T>>::insert(who, a);
        }
        None => {
            let mut a = Vec::new();
            a.push(address.clone());
            <AccountMap<T>>::insert(who, a);
        }
    }
    <AddressMap<T>>::insert(address, who.clone());
}

/// bind account
fn update_binding<T: Trait>(who: &T::AccountId, info: &TxInfo) -> bool {
    <AddressMap<T>>::get(&info.input_address).map_or_else(
        || {
            apply_update_binding::<T>(who, &info.input_address);
            let addr = info.input_address.layout().to_vec();
            Module::<T>::deposit_event(RawEvent::Bind(
                info.raw_tx.hash(),
                b58::to_base58(addr),
                who.clone(),
                BindStatus::Init,
            ));
            return true;
        },
        |p| {
            if p == *who {
                return false;
            }

            //delete old bind
            if let Some(a) = <AccountMap<T>>::get(&p) {
                let mut vaddr = a.clone();
                for (index, it) in a.iter().enumerate() {
                    if it.hash == info.input_address.hash {
                        vaddr.remove(index);
                        <AccountMap<T>>::insert(&p, vaddr);
                        break;
                    }
                }
            };
            apply_update_binding::<T>(who, &info.input_address);
            let addr = info.input_address.layout().to_vec();
            Module::<T>::deposit_event(RawEvent::Bind(
                info.raw_tx.hash(),
                b58::to_base58(addr),
                who.clone(),
                BindStatus::Update,
            ));
            return true;
        },
    )
}

fn remove_pending_deposit<T: Trait>(input_address: &keys::Address, who: &T::AccountId) {
    if let Some(record) = <PendingDepositMap<T>>::get(input_address) {
        for r in record {
            let mut balance = 0;
            <UTXOSet<T>>::mutate(r, |utxos| {
                utxos.status = true;
                balance = utxos.balance;
            });

            deposit_token::<T>(who, balance);
            runtime_io::print("[bridge-btc] handle_output PendingDepositMap ");
            runtime_io::print(balance);
        }
        <PendingDepositMap<T>>::remove(input_address);
    }
}

fn deposit_token<T: Trait>(who: &T::AccountId, balance: u64) {
    let token: xassets::Token = <Module<T> as xassets::ChainT>::TOKEN.to_vec();
    let _ = <xrecords::Module<T>>::deposit(&who, &token, As::sa(balance));
}

fn insert_pending_deposit<T: Trait>(input_address: &keys::Address, txid: H256, index: u32) {
    let k = UTXOKey { txid, index };
    match <PendingDepositMap<T>>::get(input_address) {
        Some(mut key) => {
            key.push(k);
            <PendingDepositMap<T>>::insert(input_address, key);

            runtime_io::print("[bridge-btc]（Some）handle_output PendingDeposit token: ");
        }
        None => {
            let mut cache: Vec<UTXOKey> = Vec::new();
            cache.push(k);
            <PendingDepositMap<T>>::insert(input_address, cache);

            runtime_io::print("[bridge-btc]（None）handle_output PendingDeposit token: ");
        }
    };
}

fn refresh_utxo<T: Trait>(k: UTXOKey, v: UTXOStatus) {
    <UTXOSet<T>>::insert(k.clone(), v);

    match <UTXOSetKey<T>>::take() {
        Some(mut key) => {
            key.push(k);
            <UTXOSetKey<T>>::put(key);
        }
        None => {
            let mut cache: Vec<UTXOKey> = Vec::new();
            cache.push(k);
            <UTXOSetKey<T>>::put(cache);
        }
    };
}
