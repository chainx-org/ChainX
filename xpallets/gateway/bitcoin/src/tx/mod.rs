// Copyright 2018-2019 Chainpool.

// pub mod handler;
mod secp256k1_verifier;
pub mod utils;
pub mod validator;

// Substrate
use frame_support::dispatch::DispatchError;
use sp_std::{prelude::*, result};

// ChainX
use chainx_primitives::Name;
use xpallet_gateway_common::traits::Extractable;
use xpallet_support::{debug, error, warn};

// light-bitcoin
use btc_chain::Transaction;
use btc_keys::{Address, Network};
use btc_script::Script;

// use crate::traits::RelayTransaction;
use self::utils::{
    equal_addr, inspect_address_from_transaction, is_key, parse_opreturn,
    parse_output_addr_with_networkid,
};
pub use self::validator::validate_transaction;
use crate::trustee::{get_last_trustee_address_pair, get_trustee_address_pair};
use crate::types::{AccountInfo, BTCTxResult, BTCTxState, DepositInfo, MetaTxType};
use crate::{Module, RawEvent, Trait};

pub fn process_tx<T: Trait>(
    tx: Transaction,
    prev: Option<Transaction>,
) -> result::Result<BTCTxState, DispatchError> {
    let meta_type = detect_transaction_type::<T>(&tx, prev.as_ref())?;
    // process

    let state = BTCTxState {
        result: BTCTxResult::Success,
        tx_type: Default::default(),
    };
    Ok(state)
}

pub fn detect_transaction_type<T: Trait>(
    tx: &Transaction,
    prev: Option<&Transaction>,
) -> result::Result<MetaTxType<T::AccountId>, DispatchError> {
    let addr_pair = get_trustee_address_pair::<T>()?;
    let last_addr_pair = get_last_trustee_address_pair::<T>().ok();
    let network = Module::<T>::network_id();
    let min_deposit = Module::<T>::btc_min_deposit();

    let meta_type = detect_transaction_type_impl::<T::AccountId, _>(
        tx,
        prev,
        network,
        min_deposit,
        addr_pair,
        last_addr_pair,
        |script| T::AccountExtractor::account_info(script),
    );
    Ok(meta_type)
}

/// parse tx to detect transaction type.
/// notice pass `prev` would try to detect Withdrawal|TrusteeTransition|HotAndCold types, then
/// detect deposit type. Otherwise, would just detect deposit type.
/// when type is deposit, if parse opreturn success, would use opreturn as account info, or else,
/// would use input_addr which is parsed from `prev`.
/// notice we use `AccountId, F` etc... generic type otherwise `<T: Trait>` type, is convenient for
/// bitcoin transaction relay program to reuse this part that could detect bitcoin transaction and
/// filter them before relaying to chain.
#[inline]
pub fn detect_transaction_type_impl<AccountId, F>(
    tx: &Transaction,
    prev: Option<&Transaction>,
    network: Network,
    min_deposit: u64,
    trustee_addr_pair: (Address, Address),
    old_trustee_addr_pair: Option<(Address, Address)>,
    handle_opreturn: F,
) -> MetaTxType<AccountId>
where
    F: Fn(&[u8]) -> Option<(AccountId, Option<Name>)>,
{
    let input_addr = prev.and_then(|prev_tx| {
        // parse input addr
        let outpoint = &tx.inputs[0].previous_output;
        inspect_address_from_transaction(prev_tx, outpoint, network)
    });

    // Withdrawal|TrusteeTransition|HotAndCold need input_addr to parse prev address
    if let Some(ref input_addr) = input_addr {
        /// parse tx's inputs/outputs into Option<Address>
        /// e.g
        /// notice the relay tx only has first input
        ///        _________
        ///  addr |        | Some(addr)
        ///       |   tx   | Some(addr)
        ///       |________| None (OP_RETURN or something unknown)
        // parse output addr
        let outputs: Vec<(Option<Address>, u64)> = tx
            .outputs
            .iter()
            .map(|out| {
                (
                    parse_output_addr_with_networkid(&out.script_pubkey.to_vec().into(), network),
                    out.value,
                )
            })
            .collect();
        // ---------- parse finish

        let tx_type = detect_other_type(
            &outputs,
            input_addr,
            trustee_addr_pair,
            old_trustee_addr_pair,
        );
        match tx_type {
            MetaTxType::Withdrawal | MetaTxType::TrusteeTransition | MetaTxType::HotAndCold => {
                return tx_type;
            }
            _ => {
                warn!(
                    "[detect_transaction_type_impl]|it's an irrelevance tx or deposit tx|tx_hash:{:?}",
                    tx.hash()
                );
            }
        }
    }
    // parse deposit
    let (hot_addr, _) = trustee_addr_pair;
    detect_deposit_type(
        &tx,
        min_deposit,
        &hot_addr,
        input_addr.as_ref(),
        network,
        handle_opreturn,
    )
}

fn detect_deposit_type<AccountId, F>(
    tx: &Transaction,
    min_deposit: u64,
    hot_addr: &Address,
    input_addr: Option<&Address>,
    network: Network,
    handle_opreturn: F,
) -> MetaTxType<AccountId>
where
    F: Fn(&[u8]) -> Option<(AccountId, Option<Name>)>,
{
    let (opreturn, deposit_value) =
        parse_deposit_outputs_impl(tx, hot_addr, network, handle_opreturn);
    if deposit_value >= min_deposit {
        let account_info = match opreturn {
            // if has a valid opreturn, would just use opreturn info
            Some(opreturn) => AccountInfo::OpReturn(opreturn),
            None => {
                if let Some(input) = input_addr {
                    AccountInfo::InputAddr(input.clone())
                } else {
                    warn!("[detect_deposit_type]|receive a deposit tx but do not have valid opreturn & not have input addr|tx:{:?}", tx.hash());
                    return MetaTxType::Irrelevance;
                }
            }
        };

        let info = DepositInfo {
            deposit_value,
            account_info,
        };
        MetaTxType::Deposit(info)
    } else {
        warn!("[detect_deposit_type]|receive a deposit tx but deposit value is too low, dropped|tx:{:?}", tx.hash());
        MetaTxType::Irrelevance
    }
}

fn detect_other_type<AccountId>(
    outputs: &[(Option<Address>, u64)],
    input_addr: &Address,
    trustee_addr_pair: (Address, Address),
    old_trustee_addr_pair: Option<(Address, Address)>,
) -> MetaTxType<AccountId> {
    let (hot_addr, cold_addr) = trustee_addr_pair;
    // judge tx type
    // with input_addr, allow `Withdrawal`, `Deposit`, `HotAndCold`, `TrusteeTransition`
    // judge input has trustee addr
    let input_is_trustee =
        equal_addr(&input_addr, &hot_addr) || equal_addr(&input_addr, &cold_addr);
    // judge if all outputs contains hot/cold trustee
    let all_outputs_trustee = outputs.iter().all(|(item, _)| {
        if let Some(addr) = item {
            if equal_addr(addr, &hot_addr) || equal_addr(addr, &cold_addr) {
                return true;
            }
        }
        false
    });
    // judge tx type
    if input_is_trustee {
        if all_outputs_trustee {
            return MetaTxType::HotAndCold;
        }
        // outputs contains other addr, it's user addr, thus it's a withdrawal
        return MetaTxType::Withdrawal;
    } else {
        if let Some((old_hot_addr, old_cold_addr)) = old_trustee_addr_pair {
            let input_is_old_trustee =
                equal_addr(&input_addr, &old_hot_addr) || equal_addr(&input_addr, &old_cold_addr);
            if input_is_old_trustee && all_outputs_trustee {
                // input should from old trustee addr, outputs should all be current trustee addrs
                return MetaTxType::TrusteeTransition;
            }
        }
    }
    MetaTxType::Irrelevance
}

pub fn parse_deposit_outputs_impl<AccountId, F>(
    tx: &Transaction,
    hot_addr: &Address,
    network: Network,
    handle_opreturn: F,
) -> (Option<(AccountId, Option<Name>)>, u64)
where
    F: Fn(&[u8]) -> Option<(AccountId, Option<Name>)>,
{
    let mut deposit_balance = 0;
    let mut account_info = None;
    let mut has_opreturn = false;
    // let mut original = None;
    // parse
    for output in tx.outputs.iter() {
        // out script
        let script: Script = output.script_pubkey.to_vec().into();
        // bind address [btc address --> chainx AccountId]
        // is_null_data_script is not null
        if script.is_null_data_script() {
            // only handle first valid account info opreturn, other opreturn would drop
            if has_opreturn == false {
                if let Some(v) = parse_opreturn(&script) {
                    let info = handle_opreturn(&v);
                    if info.is_some() {
                        // only set first valid account info
                        // original = Some(script.to_vec());
                        account_info = info;
                        has_opreturn = true;
                    }
                }
            }
            continue;
        }

        // not a opreturn out, do follow
        // get deposit money
        if is_key(&script, hot_addr, network) && output.value > 0 {
            deposit_balance += output.value;
        }
    }
    (account_info, deposit_balance)
}
// pub fn handle_tx<T: Trait>(txid: &H256) -> Dispatch {
//     let tx_handler = TxHandler::new::<T>(txid)?;
//     tx_handler.handle::<T>()?;
//     Ok(())
// }

// pub fn remove_unused_tx<T: Trait>(txid: &H256) {
//     debug!("[remove_unused_tx]|remove old tx|tx_hash:{:}", txid);
//     TxFor::<T>::remove(txid);
//     InputAddrFor::<T>::remove(txid);
// }

// /// Check that the cash withdrawal transaction is correct
// pub fn check_withdraw_tx<T: Trait>(tx: &Transaction, withdrawal_id_list: &[u32]) -> DispatchResult {
//     match Module::<T>::withdrawal_proposal() {
//         Some(_) => Err("Unfinished withdrawal transaction"),
//         None => {
//             // withdrawal addr list for account withdrawal application
//             let mut appl_withdrawal_list = Vec::new();
//             for withdraw_index in withdrawal_id_list.iter() {
//                 let record = xrecords::Module::<T>::application_map(withdraw_index)
//                     .ok_or("Withdraw id not in withdrawal PendingWithdrawal record")?;
//                 // record.data.addr() is base58
//                 // verify btc address would conveRelayedTx a base58 addr to Address
//                 let addr: Address = Module::<T>::verify_btc_address(&record.data.addr())
//                     .map_err(|_| "Parse addr error")?;
//                 appl_withdrawal_list.push((addr, record.data.balance().into()));
//             }
//             // not allow deposit directly to cold address, only hot address allow
//             let hot_trustee_address: Address = get_hot_trustee_address::<T>()?;
//             // withdrawal addr list for tx outputs
//             let btc_withdrawal_fee = Module::<T>::btc_withdrawal_fee();
//             let mut tx_withdraw_list = Vec::new();
//             for output in tx.outputs.iter() {
//                 let script: Script = output.script_pubkey.clone().into();
//                 let addr = parse_output_addr::<T>(&script).ok_or("not found addr in this out")?;
//                 if addr.hash != hot_trustee_address.hash {
//                     // expect change to trustee_addr output
//                     tx_withdraw_list.push((addr, output.value + btc_withdrawal_fee));
//                 }
//             }
//
//             tx_withdraw_list.soRelayedTx();
//             appl_withdrawal_list.soRelayedTx();
//
//             // appl_withdrawal_list must match to tx_withdraw_list
//             if appl_withdrawal_list.len() != tx_withdraw_list.len() {
//                 error!("withdrawal tx's outputs not equal to withdrawal application list, withdrawal application len:{:}, withdrawal tx's outputs len:{:}|withdrawal application list:{:?}, tx withdrawal outputs:{:?}",
//                        appl_withdrawal_list.len(), tx_withdraw_list.len(),
//                        withdrawal_id_list.iter().zip(appl_withdrawal_list).collect::<Vec<_>>(),
//                        tx_withdraw_list
//                 );
//                 return Err("withdrawal tx's outputs not equal to withdrawal application list");
//             }
//
//             let count = appl_withdrawal_list.iter().zip(tx_withdraw_list).filter(|(a,b)|{
//                 if a.0 == b.0 && a.1 == b.1 {
//                     return true
//                 }
//                 else {
//                     error!(
//                         "withdrawal tx's output not match to withdrawal application. withdrawal application :{:?}, tx withdrawal output:{:?}",
//                         a,
//                         b
//                     );
//                     return false
//                 }
//             }).count();
//
//             if count != appl_withdrawal_list.len() {
//                 return Err("withdrawal tx's output list not match to withdrawal application list");
//             }
//
//             Ok(())
//         }
//     }
// }
/*
/// Update the signature status of trustee
/// state: false -> Veto signature, true -> Consent signature
/// only allow inseRelayedTx once
pub fn insertTx_trustee_vote_state<T: Trait>(
    state: bool,
    who: &T::AccountId,
    trustee_list: &mut Vec<(T::AccountId, bool)>,
) -> DispatchResult {
    match trustee_list.iter_mut().find(|ref info| info.0 == *who) {
        Some(_) => {
            // if account is exist, override state
            error!("[inseRelayedTx_trustee_vote_state]|already vote for this withdrawal proposal|who:{:?}|old vote:{:}", who, state);
            return Err("already vote for this withdrawal proposal");
        }
        None => {
            trustee_list.push((who.clone(), state));
            debug!(
                "[inseRelayedTx_trustee_vote_state]|inseRelayedTx new vote|who:{:?}|state:{:}",
                who, state
            );
        }
    }
    Module::<T>::deposit_event(RawEvent::SignWithdrawalProposal(who.clone(), state));
    Ok(())
}
*/
