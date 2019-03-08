// Copyright 2019 Chainpool

use xr_primitives::generic::{b58, Extracter};
use xr_primitives::traits::Extractable;

use crate::btc_chain::Transaction;
use crate::btc_keys::{Address, DisplayLayout};
use crate::btc_primitives::hash::H256;
use crate::btc_script::script::Script;
use crate::rstd::prelude::Vec;
use crate::rstd::result::Result as StdResult;
use crate::xaccounts;
use crate::xassets::Chain;

use crate::runtime_primitives::traits::As;
use crate::support::{dispatch::Result, StorageMap, StorageValue};
use crate::types::{DepositAccountInfo, DepositCache, TxInfo, TxType};
use crate::xassets::ChainT;
use crate::{xassets, xrecords};
use crate::{Module, PendingDepositMap, RawEvent, Trait, WithdrawalFatalErr, WithdrawalProposal};

#[cfg(feature = "std")]
use crate::hash_strip;
#[cfg(feature = "std")]
use crate::xsupport::u8array_to_string;
use crate::xsupport::{debug, error, info};

use super::utils::{get_trustee_address, is_key};

pub struct TxHandler {
    pub tx_hash: H256,
    pub tx_info: TxInfo,
}

impl TxHandler {
    pub fn new<T: Trait>(txid: &H256) -> StdResult<TxHandler, &'static str> {
        let tx_info = Module::<T>::tx_for(txid).ok_or("not find this txinfo for this txid")?;
        Ok(TxHandler {
            tx_hash: txid.clone(),
            tx_info,
        })
    }

    pub fn handle<T: Trait>(&self) -> Result {
        match self.tx_info.tx_type {
            TxType::Withdraw => {
                // TODO refactor
                self.withdraw::<T>()?;
            }
            TxType::Deposit => {
                self.deposit::<T>()?;
            }
        };
        Ok(())
    }

    fn withdraw<T: Trait>(&self) -> Result {
        if let Some(proposal) = WithdrawalProposal::<T>::take() {
            debug!(
                "[withdraw]|withdraw handle|proposal:{:?}|tx:{:?}",
                proposal, self.tx_info.raw_tx
            );
            match ensure_identical(&self.tx_info.raw_tx, &proposal.tx) {
                Ok(()) => {
                    for number in proposal.withdrawal_id_list.iter() {
                        match xrecords::Module::<T>::withdrawal_finish(*number, true) {
                            Ok(_) => {
                                info!("[withdraw]|ID of withdrawal completion: {:}", *number);
                            }
                            Err(_e) => {
                                error!("[withdraw]|ID of withdrawal ERROR! {:}, reason:{:}, please use root to fix it", *number, _e);
                            }
                        }
                        Module::<T>::deposit_event(RawEvent::Withdrawal(
                            *number,
                            self.tx_hash.to_vec(),
                            xrecords::TxState::Confirmed,
                        ));
                    }
                }
                Err(e) => {
                    let tx_hash = proposal.tx.hash();
                    error!("[withdraw]|Withdrawal failed, reason:{:}, please use root to fix it|withdrawal idlist:{:?}|proposal:{:?}|tx:{:?}|tx hash:{:}",
                           e, proposal.withdrawal_id_list, proposal.tx, self.tx_info.raw_tx, self.tx_hash);
                    WithdrawalProposal::<T>::put(proposal);

                    WithdrawalFatalErr::<T>::put(true);

                    Module::<T>::deposit_event(RawEvent::NeedDropWithdrawTx(
                        tx_hash.to_vec(),
                        self.tx_hash.to_vec(),
                    ));
                    return Err(e);
                }
            };
        }
        Ok(())
    }

    fn deposit<T: Trait>(&self) -> Result {
        let (account_info, deposit_balance, original_opretion) =
            parse_deposit_outputs::<T>(&self.tx_info.raw_tx)?;

        debug!(
            "[deposit]|parse outputs|account_info:{:?}|balance:{:}|opreturn:{:}|",
            account_info,
            deposit_balance,
            if original_opretion.len() > 2 {
                format!(
                    "{:?}|{:}",
                    original_opretion[..2].to_vec(),
                    u8array_to_string(&original_opretion[2..])
                )
            } else {
                u8array_to_string(&original_opretion)
            }
        );

        let input_addr: Address = if let Some(addr) = Module::<T>::input_addr_for(&self.tx_hash) {
            addr
        } else {
            error!(
                "[deposit]|deposit tx must have input addr|tx:{:?}",
                self.tx_info
            );
            panic!("must set input addr before");
        };

        // get accounid from related info
        let deposit_account_info: DepositAccountInfo<T::AccountId> =
            if let Some((accountid, channel_name)) = account_info {
                // remove old unbinding deposit info
                remove_pending_deposit::<T>(&input_addr, &accountid);
                // update or override binding info
                update_binding::<T>(accountid.clone(), channel_name, input_addr.clone());
                DepositAccountInfo::AccountId(accountid)
            } else {
                // no opreturn, use addr to get accountid
                let key = (Chain::Bitcoin, input_addr.layout().to_vec());
                match xaccounts::Module::<T>::address_map(&key) {
                    Some((accountid, _)) => DepositAccountInfo::AccountId(accountid),
                    None => DepositAccountInfo::Address(input_addr.clone()),
                }
            };
        // deposit
        if deposit_balance > 0 {
            // deposit for this account or store this deposit cache
            let deposit_account = match deposit_account_info {
                DepositAccountInfo::AccountId(accountid) => {
                    deposit_token::<T>(&accountid, deposit_balance);
                    info!(
                        "[deposit]|deposit success|who:{:}|balance:{:}|tx_hash:{:}...",
                        accountid,
                        deposit_balance,
                        hash_strip(&self.tx_hash)
                    );
                    accountid
                }
                DepositAccountInfo::Address(addr) => {
                    insert_pending_deposit::<T>(&addr, &self.tx_hash, deposit_balance);
                    info!(
                        "[deposit]|deposit into pending|addr:{:?}|balance:{:}|tx_hash:{:}...",
                        addr,
                        deposit_balance,
                        hash_strip(&self.tx_hash)
                    );
                    Default::default()
                }
            };

            Module::<T>::deposit_event(RawEvent::Deposit(
                deposit_account,
                xassets::Chain::Bitcoin,
                Module::<T>::TOKEN.to_vec(),
                As::sa(deposit_balance),
                self.tx_hash.to_vec(),
                original_opretion,
                b58::to_base58(input_addr.layout().to_vec()),
                xrecords::TxState::Confirmed,
            ));
        }
        Ok(())
    }
}

/// Try updating the binding address, remove pending deposit if the updating goes well.
/// return validator name and this accountid
fn handle_opreturn<T: Trait>(script: &[u8]) -> Option<(T::AccountId, Vec<u8>)> {
    Extracter::<T::AccountId>::new(script.to_vec()).account_info()
}

pub fn parse_deposit_outputs<T: Trait>(
    tx: &Transaction,
) -> StdResult<(Option<(T::AccountId, Vec<u8>)>, u64, Vec<u8>), &'static str> {
    let trustee_address = get_trustee_address::<T>()?;
    let mut deposit_balance = 0;
    let mut account_info = None;
    let mut has_opreturn = false;
    let mut original = Vec::new();
    // parse
    for output in tx.outputs.iter() {
        // out script
        let script: Script = output.script_pubkey.to_vec().into();
        // bind address [btc address --> chainx AccountId]
        // is_null_data_script is not null
        if script.is_null_data_script() {
            // TODO check is_null_data_script
            if has_opreturn == false {
                // only handle first opreturn output
                // OP_CODE PUSH ... (2 BYTES)
                account_info = handle_opreturn::<T>(&script[2..]);
                if account_info.is_some() {
                    original.extend(script.to_vec());
                }
                has_opreturn = true;
            }
            continue;
        }

        // get deposit money
        if is_key::<T>(&script, &trustee_address) && output.value > 0 {
            deposit_balance += output.value;
        }
    }
    Ok((account_info, deposit_balance, original))
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
                error!(
                    "[ensure_identical]|tx1 not equal to tx2|tx1:{:?}|tx2:{:?}",
                    tx1, tx2
                );
                return Err("The inputs of these two transactions mismatch.");
            }
        }
        return Ok(());
    }
    Err("The transaction text does not match the original text to be signed.")
}

/// bind account
fn update_binding<T: Trait>(who: T::AccountId, channel_name: Vec<u8>, input_addr: Address) {
    // override old binding
    xaccounts::apply_update_binding::<T>(
        who,
        (Chain::Bitcoin, input_addr.layout().to_vec()),
        channel_name,
    );
}

pub fn deposit_token<T: Trait>(who: &T::AccountId, balance: u64) {
    let token: xassets::Token = <Module<T> as xassets::ChainT>::TOKEN.to_vec();
    let _ = <xrecords::Module<T>>::deposit(&who, &token, As::sa(balance)).map_err(|e| {
        error!(
            "call xrecores to deposit error!, must use root to fix this error. reason:{:?}",
            e
        );
        e
    });
}

fn insert_pending_deposit<T: Trait>(input_address: &keys::Address, txid: &H256, balance: u64) {
    let cache = DepositCache {
        txid: txid.clone(),
        balance,
    };

    match Module::<T>::pending_deposit(input_address) {
        Some(mut list) => {
            if !list.contains(&cache) {
                list.push(cache);
            }
            PendingDepositMap::<T>::insert(input_address, list);
            info!(
                "Add pending deposit: {:}...  {:}",
                hash_strip(txid),
                balance
            );
        }
        None => {
            let mut list: Vec<DepositCache> = Vec::new();
            list.push(cache);
            PendingDepositMap::<T>::insert(input_address, list);
            info!(
                "New pending deposit: {:}...  {:}",
                hash_strip(txid),
                balance
            );
        }
    };
}

fn remove_pending_deposit<T: Trait>(input_address: &keys::Address, who: &T::AccountId) {
    if let Some(record) = Module::<T>::pending_deposit(input_address) {
        for r in record {
            deposit_token::<T>(who, r.balance);
            info!(
                "[remove_pending_deposit]|use pending info to re-deposit|who:{:}|balance:{:}",
                who, r.balance
            );

            Module::<T>::deposit_event(RawEvent::DepositPending(
                who.clone(),
                xassets::Chain::Bitcoin,
                Module::<T>::TOKEN.to_vec(),
                As::sa(r.balance),
                b58::to_base58(input_address.layout().to_vec()),
            ));
        }
        PendingDepositMap::<T>::remove(input_address);
    }
}
