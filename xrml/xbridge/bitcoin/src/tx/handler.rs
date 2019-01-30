use super::*;
use keys::DisplayLayout;
use xaccounts;
use xassets::Chain;
use xr_primitives::generic::Extracter;
use xr_primitives::traits::Extractable;
pub struct TxHandler<'a>(&'a H256);

impl<'a> TxHandler<'a> {
    pub fn new(txid: &H256) -> TxHandler {
        TxHandler(txid)
    }

    pub fn withdraw<T: Trait>(&self) -> Result {
        runtime_io::print("[bridge-btc] handle_withdraw start");

        //delete used
        let txid = self.0;
        let tx_info = <TxFor<T>>::get(txid);
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
        Module::<T>::deposit_event(RawEvent::WithdrawTx(
            tx_info.raw_tx.hash(),
            tx_info.input_address.layout().to_vec(),
            flag,
        ));
        Ok(())
    }

    pub fn deposit<T: Trait>(&self, trustee_address: &keys::Address) {
        runtime_io::print("[bridge-btc] handle_output start");
        let mut deposit_balance = 0;
        let txid = self.0;
        let tx_info = <TxFor<T>>::get(txid);

        for (_index, output) in tx_info.raw_tx.outputs.iter().enumerate() {
            let script = &output.script_pubkey;
            let into_script: Script = script.clone().into();

            // bind address [btc address --> chainx AccountId]
            if into_script.is_null_data_script() {
                let s = script.clone();
                handle_opreturn::<T>(&s[2..], &tx_info);
                continue;
            }

            // get deposit money
            let script_addresses = into_script.extract_destinations().unwrap_or(Vec::new());
            if script_addresses.len() == 1 {
                if (trustee_address.hash == script_addresses[0].hash) && (output.value > 0) {
                    deposit_balance += output.value;
                }
            }
        }

        if deposit_balance > 0 {
            let mut deposit_status = false;
            let input_address = tx_info.input_address.clone();
            <xaccounts::CrossChainAddressMapOf<T>>::get((Chain::Bitcoin, input_address.layout().to_vec()))
                .map_or_else(
                    || insert_pending_deposit::<T>(&input_address, txid.clone(), deposit_balance),
                    |a| {
                        deposit_token::<T>(&a.0, deposit_balance);
                        runtime_io::print("[bridge-btc] handle_output deposit_token: ");
                        deposit_status = true;
                    },
                );

            let addr = tx_info.input_address.layout().to_vec();
            runtime_io::print(deposit_balance);
            Module::<T>::deposit_event(RawEvent::Deposit(
                tx_info.raw_tx.hash(),
                b58::to_base58(addr),
                deposit_balance,
                deposit_status,
            ));
        }
    }
}

/// Try updating the binding address, remove pending deposit if the updating goes well.
fn handle_opreturn<T: Trait>(script: &[u8], info: &TxInfo) {
    if let Some(a) = Extracter::<T::AccountId>::new(script.to_vec()).account_info() {
        if update_binding::<T>(a.0, a.1.clone(), info) {
            runtime_io::print("[bridge-btc] handle_output register ");
            remove_pending_deposit::<T>(&info.input_address, &a.1);
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
fn apply_update_binding<T: Trait>(who: T::AccountId, channle_id: T::AccountId, address: Vec<u8>) {
    //let chain_id = Module::<T>::chain();

    match <xaccounts::CrossChainBindOf<T>>::get((Chain::Bitcoin, who.clone())) {
        Some(mut a) => {
            a.push(address.clone());
            <xaccounts::CrossChainBindOf<T>>::insert((Chain::Bitcoin, who.clone()), a);
        }
        None => {
            let mut a = Vec::new();
            a.push(address.clone());
            <xaccounts::CrossChainBindOf<T>>::insert((Chain::Bitcoin, who.clone()), a);
        }
    }
    <xaccounts::CrossChainAddressMapOf<T>>::insert((Chain::Bitcoin, address), (who.clone(), channle_id));
}

/// bind account
fn update_binding<T: Trait>(node_name: Vec<u8>, who: T::AccountId, info: &TxInfo) -> bool {
    let input_addr = info.input_address.layout().to_vec();
    <xaccounts::CrossChainAddressMapOf<T>>::get((Chain::Bitcoin, input_addr.clone())).map_or_else(
        || {
            let channle_id =
                <xaccounts::IntentionOf<T>>::get(node_name.clone()).unwrap_or_default();
            apply_update_binding::<T>(who.clone(), channle_id.clone(), input_addr.clone());
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
            let channle_id =
                <xaccounts::IntentionOf<T>>::get(node_name.clone()).unwrap_or_default();
            if p.0 == who.clone() && p.1 == channle_id.clone() {
                return false;
            }
            //delete old bind
            if let Some(a) = <xaccounts::CrossChainBindOf<T>>::get((Chain::Bitcoin, p.0)) {
                let mut vaddr = a.clone();
                for (index, it) in a.iter().enumerate() {
                    let addr: Address =
                        Decode::decode(&mut it.as_slice()).unwrap_or(Default::default());
                    if addr.hash == info.input_address.hash {
                        vaddr.remove(index);
                        <xaccounts::CrossChainBindOf<T>>::insert((Chain::Bitcoin, who.clone()), vaddr);
                        break;
                    }
                }
            };
            apply_update_binding::<T>(who.clone(), channle_id, input_addr.clone());
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
            deposit_token::<T>(who, r.balance);
            runtime_io::print("[bridge-btc] handle_output PendingDepositMap ");
            runtime_io::print(r.balance);
        }
        <PendingDepositMap<T>>::remove(input_address);
    }
}

fn deposit_token<T: Trait>(who: &T::AccountId, balance: u64) {
    let token: xassets::Token = <Module<T> as xassets::ChainT>::TOKEN.to_vec();
    let _ = <xrecords::Module<T>>::deposit(&who, &token, As::sa(balance));
}

fn insert_pending_deposit<T: Trait>(input_address: &keys::Address, txid: H256, balance: u64) {
    let k = DepositCache { txid, balance };
    match <PendingDepositMap<T>>::get(input_address) {
        Some(mut key) => {
            key.push(k);
            <PendingDepositMap<T>>::insert(input_address, key);

            runtime_io::print("[bridge-btc]（Some）handle_output PendingDeposit token: ");
        }
        None => {
            let mut cache: Vec<DepositCache> = Vec::new();
            cache.push(k);
            <PendingDepositMap<T>>::insert(input_address, cache);

            runtime_io::print("[bridge-btc]（None）handle_output PendingDeposit token: ");
        }
    };
}
