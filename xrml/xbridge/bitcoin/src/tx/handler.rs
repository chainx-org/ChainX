// Copyright 2018-2019 Chainpool.

// Substrate
use primitives::traits::As;
use rstd::{prelude::Vec, result};
use support::{dispatch::Result, StorageMap, StorageValue};

// ChainX
use xr_primitives::{generic::b58, Name};

use xassets::{self, ChainT};
use xbridge_common::traits::{CrossChainBinding, Extractable};
use xfee_manager;

use xrecords;
#[cfg(feature = "std")]
use xsupport::u8array_to_string;
use xsupport::{debug, error, info};

// light-bitcoin
use btc_chain::Transaction;
use btc_keys::{Address, DisplayLayout};
use btc_primitives::H256;
use btc_script::Script;

use crate::types::{DepositAccountInfo, DepositCache, TxInfo, TxType};
use crate::{CurrentWithdrawalProposal, Module, PendingDepositMap, RawEvent, Trait, TxFor};

use super::utils::{ensure_identical, get_hot_trustee_address, is_key};

pub struct TxHandler {
    pub tx_hash: H256,
    pub tx_info: TxInfo,
}

impl TxHandler {
    pub fn new<T: Trait>(txid: &H256) -> result::Result<TxHandler, &'static str> {
        let tx_info = Module::<T>::tx_for(txid).ok_or("not find this txinfo for this txid")?;
        if tx_info.done {
            error!(
                "[TxHandler]|this tx has already been handled|tx_hash:{:}",
                txid
            );
            return Err("this tx has already been handled");
        }

        Ok(TxHandler {
            tx_hash: txid.clone(),
            tx_info,
        })
    }

    pub fn handle<T: Trait>(&self) -> Result {
        match self.tx_info.tx_type {
            TxType::Withdrawal => {
                // TODO refactor
                self.withdraw::<T>()?;
            }
            TxType::Deposit => {
                self.deposit::<T>()?;
            }
            _ => {
                info!(
                    "[handle tx]|other type tx|type:{:?}|hash:{:?}|tx:{:?}",
                    self.tx_info.tx_type, self.tx_hash, self.tx_info.raw_tx
                );
            }
        };

        // handle finish, mark this tx has done
        let mut tx_info = self.tx_info.clone();
        tx_info.done = true;
        TxFor::<T>::insert(&self.tx_hash, tx_info);

        Ok(())
    }

    fn withdraw<T: Trait>(&self) -> Result {
        if let Some(proposal) = CurrentWithdrawalProposal::<T>::take() {
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
                            self.tx_hash.as_bytes().to_vec(),
                            xrecords::TxState::Confirmed,
                        ));
                    }
                }
                Err(e) => {
                    let tx_hash = proposal.tx.hash();
                    error!("[withdraw]|Withdrawal failed, reason:{:}, please use root to fix it|withdrawal idlist:{:?}|proposal:{:?}|tx:{:?}|tx hash:{:}",
                           e, proposal.withdrawal_id_list, proposal.tx, self.tx_info.raw_tx, self.tx_hash);
                    CurrentWithdrawalProposal::<T>::put(proposal);

                    Module::<T>::deposit_event(RawEvent::WithdrawalFatalErr(
                        self.tx_hash.as_bytes().to_vec(),
                        tx_hash.as_bytes().to_vec(),
                    ));

                    xfee_manager::Switch::<T>::mutate(|switch| {
                        switch.xbtc = true;
                    });

                    return Err(e);
                }
            };
        } else {
            error!("[withdraw]|Withdrawal failed, the proposal is EMPTY, but receive a withdrawal tx, please use root to fix it|tx:{:?}|tx hash:{:}", self.tx_info.raw_tx, self.tx_hash);

            // no proposal, but find a withdraw tx, it's a fatal error in withdrawal
            Module::<T>::deposit_event(RawEvent::WithdrawalFatalErr(
                self.tx_hash.as_bytes().to_vec(),
                Default::default(),
            ));

            xfee_manager::Switch::<T>::mutate(|switch| {
                switch.xbtc = true;
            });
        }
        Ok(())
    }

    fn deposit<T: Trait>(&self) -> Result {
        // check first input
        let input_addr: Address = Module::<T>::input_addr_for(&self.tx_hash)
            .ok_or_else(|| {
                error!(
                    "[deposit]|deposit tx must have input addr|tx:{:?}",
                    self.tx_info
                );
                ""
            })
            .expect("must set input addr before; qed");

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

        // get accounid from related info
        let deposit_account_info: DepositAccountInfo<T::AccountId> =
            if let Some((accountid, channel_name)) = account_info {
                // remove old unbinding deposit info
                remove_pending_deposit::<T>(&input_addr, &accountid);
                // update or override binding info
                update_binding::<T>(&accountid, channel_name, input_addr.clone());
                DepositAccountInfo::AccountId(accountid)
            } else {
                // no opreturn, use addr to get accountid
                match T::CrossChainProvider::get_binding_info(&input_addr) {
                    Some((accountid, _)) => DepositAccountInfo::AccountId(accountid),
                    None => DepositAccountInfo::Address(input_addr.clone()),
                }
            };
        // deposit

        // deposit for this account or store this deposit cache
        let deposit_account = match deposit_account_info {
            DepositAccountInfo::AccountId(accountid) => {
                if deposit_balance > 0 {
                    deposit_token::<T>(&accountid, deposit_balance);
                    info!(
                        "[deposit]|deposit success|who:{:?}|balance:{:}|tx_hash:{:}",
                        accountid, deposit_balance, self.tx_hash
                    );
                } else {
                    info!(
                        "[deposit]|deposit balance is 0, may be a binding|who:{:?}",
                        accountid
                    );
                }
                accountid
            }
            DepositAccountInfo::Address(addr) => {
                if deposit_balance > 0 {
                    insert_pending_deposit::<T>(&addr, &self.tx_hash, deposit_balance);
                    info!(
                        "[deposit]|deposit into pending|addr:{:?}|balance:{:}|tx_hash:{:}",
                        addr, deposit_balance, self.tx_hash
                    );
                } else {
                    error!("[deposit]|the deposit balance is 0, but not get binding info from opreturn, maybe it's not a related tx|tx:{:?}|txinfo:{:?}", self.tx_hash, self.tx_info);
                }
                Default::default()
            }
        };

        Module::<T>::deposit_event(RawEvent::Deposit(
            deposit_account,
            xassets::Chain::Bitcoin,
            Module::<T>::TOKEN.to_vec(),
            As::sa(deposit_balance),
            original_opretion,
            b58::to_base58(input_addr.layout().to_vec()),
            self.tx_hash.as_bytes().to_vec(),
            xrecords::TxState::Confirmed,
        ));
        Ok(())
    }
}

/// Try updating the binding address, remove pending deposit if the updating goes well.
/// return validator name and this accountid
fn handle_opreturn<T: Trait>(script: &[u8], addr_type: u8) -> Option<(T::AccountId, Option<Name>)> {
    T::AccountExtractor::account_info(script, addr_type)
}

pub fn parse_deposit_outputs<T: Trait>(
    tx: &Transaction,
) -> result::Result<(Option<(T::AccountId, Option<Name>)>, u64, Vec<u8>), &'static str> {
    let trustee_address = get_hot_trustee_address::<T>()?;
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
            if has_opreturn == false {
                // only handle first opreturn output
                // OP_CODE PUSH ... (2 BYTES)
                let addr_type = xsystem::Module::<T>::address_type();
                account_info = handle_opreturn::<T>(&script[2..], addr_type);
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

/// bind account
fn update_binding<T: Trait>(who: &T::AccountId, channel_name: Option<Name>, input_addr: Address) {
    T::CrossChainProvider::update_binding(who, input_addr, channel_name)
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

fn insert_pending_deposit<T: Trait>(input_address: &Address, txid: &H256, balance: u64) {
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
                "[insert_pending_deposit]|Add pending deposit|txhash:{:}|balance:{:}",
                txid, balance
            );
        }
        None => {
            let mut list: Vec<DepositCache> = Vec::new();
            list.push(cache);
            PendingDepositMap::<T>::insert(input_address, list);
            info!(
                "[insert_pending_deposit]|New pending deposit|txhash:{:}|balance:{:}",
                txid, balance
            );
        }
    };
}

fn remove_pending_deposit<T: Trait>(input_address: &Address, who: &T::AccountId) {
    if let Some(record) = Module::<T>::pending_deposit(input_address) {
        for r in record {
            deposit_token::<T>(who, r.balance);
            info!(
                "[remove_pending_deposit]|use pending info to re-deposit|who:{:?}|balance:{:}",
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
