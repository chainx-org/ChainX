// Copyright 2018-2019 Chainpool.

// Substrate
use frame_support::{
    debug::native,
    dispatch::{DispatchError, DispatchResult},
};
use sp_runtime::traits::SaturatedConversion;
use sp_std::{prelude::Vec, result};

// ChainX
use chainx_primitives::AssetId;
// use xbridge_common::traits::{CrossChainBinding, Extractable};
// use xfee_manager;
use xpallet_assets::{self, ChainT};
use xpallet_support::{debug, error, info, try_hex, warn, RUNTIME_TARGET};

// light-bitcoin
use btc_chain::Transaction;
use btc_keys::Address;
use btc_primitives::H256;
use btc_script::Script;

use crate::types::{BTCTxInfo, DepositAccountInfo, DepositCache};
use crate::{Module, RawEvent, Trait, TxMarkFor};

use super::utils::{addr2vecu8, ensure_identical, is_key, parse_opreturn};

pub struct TxHandler {
    pub tx_hash: H256,
    pub tx_info: BTCTxInfo,
}

impl TxHandler {
    pub fn new<T: Trait>(tx_info: BTCTxInfo) -> TxHandler {
        TxHandler {
            tx_hash: tx_info.raw_tx.hash(),
            tx_info,
        }
    }

    pub fn handle<T: Trait>(&self) -> DispatchResult {
        native::debug!(
            target: RUNTIME_TARGET,
            "[TxHandler]|handle tx|type:{:?}|hash:{:}|tx:{:?}",
            self.tx_info.tx_type,
            self.tx_hash,
            self.tx_info.raw_tx
        );
        // if err, do no mark this tx has been handled
        /*
                match self.tx_info.tx_type {
                    TxType::Withdrawal => {
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
        */
        // handle finish, mark this tx has done
        // TxMarkFor::insert(&self.tx_hash, ());

        Ok(())
    }

    fn withdraw<T: Trait>(&self) -> DispatchResult {
        /*
        if let Some(proposal) = CurrentWithdrawalProposal::<T>::take() {
            debug!(
                "[withdraw]|withdraw handle|proposal:{:?}|tx:{:?}",
                proposal, self.tx_info.raw_tx
            );
            match ensure_identical(&self.tx_info.raw_tx, &proposal.tx) {
                Ok(()) => {
                    for number in proposal.withdrawal_id_list.iter() {
                        match xpallet_gateway_records::Module::<T>::withdrawal_finish(*number) {
                            Ok(_) => {
                                info!("[withdraw]|ID of withdrawal completion: {:}", *number);
                            }
                            Err(_e) => {
                                error!("[withdraw]|ID of withdrawal ERROR! {:}, reason:{:}, please use root to fix it", *number, _e);
                            }
                        }
                        // Module::<T>::deposit_event(RawEvent::Withdrawal(
                        //     *number,
                        //     self.tx_hash.as_bytes().to_vec(),
                        //     xrecords::TxState::Confirmed,
                        // ));
                    }
                }
                Err(e) => {
                    let tx_hash = proposal.tx.hash();
                    error!("[withdraw]|Withdrawal failed, reason:{:}, please use root to fix it|withdrawal idlist:{:?}|proposal:{:?}|tx:{:?}|tx hash:{:}",
                           e, proposal.withdrawal_id_list, proposal.tx, self.tx_info.raw_tx, self.tx_hash);
                    // CurrentWithdrawalProposal::<T>::put(proposal);

                    Module::<T>::deposit_event(RawEvent::WithdrawalFatalErr(
                        self.tx_hash.as_bytes().to_vec(),
                        tx_hash.as_bytes().to_vec(),
                    ));

                    // let _ = xfee_manager::Module::<T>::modify_switcher(
                    //     xfee_manager::CallSwitcher::XBTC,
                    //     true,
                    // );

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

            // TODO use trait
            // let _ =
            //     xfee_manager::Module::<T>::modify_switcher(xfee_manager::CallSwitcher::XBTC, true);
            // do not return Err, mark this tx has been handled
        }*/
        Ok(())
    }

    fn deposit<T: Trait>(&self) -> DispatchResult {
        // try to get check first input for this deposit tx
        // let input_addr: Option<Address> = Module::<T>::input_addr_for(&self.tx_hash);
        /*
        // parse deposit account info from opreturn
        let (account_info, deposit_balance, original_opreturn) =
            parse_deposit_outputs::<T>(&self.tx_info.raw_tx)?;
        let original_opreturn = original_opreturn.unwrap_or_default();

        native::debug!(
            target: RUNTIME_TARGET,
            "[deposit]|parse outputs|account_info:{:?}|balance:{:}|opreturn:{:}|",
            account_info,
            deposit_balance,
            trick_print_opreturn(&original_opreturn)
        );

        // get accounid from related info, judge accountinfo is accountid or address
        let deposit_account_info: DepositAccountInfo<T::AccountId> = match account_info {
            Some((accountid, channel_name)) => {
                if let Some(addr) = input_addr {
                    // remove old unbinding deposit info
                    remove_pending_deposit::<T>(&addr, &accountid);
                    // update or override binding info
                    // TODO
                    // update_binding::<T>(&accountid, channel_name, addr.clone());
                } else {
                    // no input addr
                    warn!("[deposit]|no input addr for this deposit tx, but has opreturn to get accountid|tx_hash:{:?}|who:{:?}", self.tx_hash, accountid);
                }
                DepositAccountInfo::AccountId(accountid)
            },
            None => {
                if let Some(addr) = input_addr {
                    // no opreturn, use addr to get accountid
                    match T::CrossChainProvider::get_binding_info(&addr) {
                        Some((accountid, _)) => DepositAccountInfo::AccountId(accountid),
                        None => DepositAccountInfo::Address(addr.clone()),
                    }
                } else {
                    error!("[deposit]|no input addr for this deposit tx, neither has opreturn to get accountid!|tx_hash:{:?}", self.tx_hash);
                    return Err("should not happen, no input addr for this deposit tx, neither has opreturn to get accountid");
                }
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
            xpallet_assets::Chain::Bitcoin,
            Module::<T>::ASSET_ID,
            deposit_balance.into(),
            original_opreturn,
            input_addr.map(|addr| addr2vecu8(&addr)).unwrap_or_default(), // unwrap is no input addr
            self.tx_hash.as_bytes().to_vec(),
            xrecords::TxState::Confirmed,
        ));*/
        Ok(())
    }
}

/// Try updating the binding address, remove pending deposit if the updating goes well.
/// return validator name and this accountid
fn handle_opreturn<T: Trait>(
    script: &[u8],
    addr_type: u8,
) -> Option<(T::AccountId, Option<Vec<u8>>)> {
    // T::AccountExtractor::account_info(script, addr_type)
    // TODO
    None
}

// pub fn parse_deposit_outputs<T: Trait>(
//     tx: &Transaction,
// ) -> Option<(T::AccountId, Option<Vec<u8>>, u64, Script)> {
//     let trustee_address = get_hot_trustee_address::<T>()?;
//     parse_deposit_outputs_impl::<T>(tx, &trustee_address)
// }

// just for test easy
#[inline]
pub fn parse_deposit_outputs_impl<T: Trait>(
    tx: &Transaction,
    hot_addr: &Address,
) -> Option<(T::AccountId, Option<Vec<u8>>, u64, Script)> {
    // let mut deposit_balance = 0;
    // let mut account_info = None;
    // let mut has_opreturn = false;
    // let mut original = None;

    // parse
    // just find first matched opreturn
    // let account_info = tx.outputs.iter().find_map(|output| {
    //     let script: Script = output.script_pubkey.to_vec().into();
    //     if let Some(v) = parse_opreturn(&script) {
    //         let addr_type = xsystem::Module::<T>::address_type();
    //         handle_opreturn::<T>(&v, addr_type).map(|(accountid, name)| (accountid, name, script))
    //     } else {
    //         None
    //     }
    // });
    //
    // account_info.map(| (accountid, name, script)| {
    //     let deposit_value = tx.outputs.iter().filter_map(|output| {
    //         if is_key::<T>(&script, hot_addr) && output.value > 0 {
    //             Some(output.value)
    //         } else {
    //             None
    //         }
    //     }).sum();
    //     (accountid, name, deposit_value, script)
    // })
    // for output in tx.outputs.iter() {
    //     // out script
    //     let script: Script = output.script_pubkey.to_vec().into();
    //     // bind address [btc address --> chainx AccountId]
    //     // is_null_data_script is not null
    //     if script.is_null_data_script() {
    //         // only handle first valid account info opreturn, other opreturn would drop
    //         if has_opreturn == false {
    //             if let Some(v) = parse_opreturn(&script) {
    //                 let addr_type = xsystem::Module::<T>::address_type();
    //                 let info = handle_opreturn::<T>(&v, addr_type);
    //                 if info.is_some() {
    //                     // only set first valid account info
    //                     original = Some(script.to_vec());
    //                     account_info = info;
    //                     has_opreturn = true;
    //                 }
    //             }
    //         }
    //         continue;
    //     }
    //
    //     // not a opreturn out, do follow
    //     // get deposit money
    //     if is_key::<T>(&script, hot_addr) && output.value > 0 {
    //         deposit_balance += output.value;
    //     }
    // }
    // Ok((account_info, deposit_balance, original))
    None
}

/// bind account
// fn update_binding<T: Trait>(who: &T::AccountId, channel_name: Option<Name>, input_addr: Address) {
//     T::CrossChainProvider::update_binding(who, input_addr, channel_name)
// }

pub fn deposit_token<T: Trait>(who: &T::AccountId, balance: u64) {
    let id: AssetId = <Module<T> as xpallet_assets::ChainT>::ASSET_ID;

    let b: T::Balance = balance.saturated_into();
    let _ = <xpallet_gateway_records::Module<T>>::deposit(&who, &id, b).map_err(|e| {
        error!(
            "deposit error!, must use root to fix this error. reason:{:?}",
            e
        );
        e
    });
}

// fn insert_pending_deposit<T: Trait>(input_address: &Address, txid: &H256, balance: u64) {
//     let cache = DepositCache {
//         txid: txid.clone(),
//         balance,
//     };
//
//     match Module::<T>::pending_deposit(input_address) {
//         Some(mut list) => {
//             if !list.contains(&cache) {
//                 list.push(cache);
//             }
//             PendingDepositMap::<T>::insert(input_address, list);
//             info!(
//                 "[insert_pending_deposit]|Add pending deposit|txhash:{:}|balance:{:}",
//                 txid, balance
//             );
//         }
//         None => {
//             let mut list: Vec<DepositCache> = Vec::new();
//             list.push(cache);
//             PendingDepositMap::<T>::insert(input_address, list);
//             info!(
//                 "[insert_pending_deposit]|New pending deposit|txhash:{:}|balance:{:}",
//                 txid, balance
//             );
//         }
//     };
// }

// pub fn remove_pending_deposit<T: Trait>(input_address: &Address, who: &T::AccountId) {
//     if let Some(record) = Module::<T>::pending_deposit(input_address) {
//         for r in record {
//             deposit_token::<T>(who, r.balance);
//             info!(
//                 "[remove_pending_deposit]|use pending info to re-deposit|who:{:?}|balance:{:}",
//                 who, r.balance
//             );
//
//             Module::<T>::deposit_event(RawEvent::DepositPending(
//                 who.clone(),
//                 xpallet_assets::Chain::Bitcoin,
//                 Module::<T>::ASSET_ID,
//                 r.balance.into(),
//                 addr2vecu8(input_address),
//             ));
//         }
//         PendingDepositMap::<T>::remove(input_address);
//     }
// }

#[cfg(feature = "std")]
#[inline]
fn trick_print_opreturn(opreturn: &[u8]) -> String {
    if opreturn.len() > 2 {
        // trick, just for print log
        format!("{:?}|{:?}", &opreturn[..2], try_hex!(&opreturn[2..]))
    } else {
        format!("{:?}", opreturn)
    }
}
