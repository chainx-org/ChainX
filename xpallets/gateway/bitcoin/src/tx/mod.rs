// Copyright 2018-2019 Chainpool.

mod secp256k1_verifier;
pub mod utils;
pub mod validator;

// Substrate
use frame_support::{
    debug::native,
    dispatch::{DispatchError, DispatchResult},
    StorageMap, StorageValue,
};
use sp_runtime::SaturatedConversion;
use sp_std::{fmt::Debug, prelude::*, result};
// ChainX
use chainx_primitives::{AssetId, Name};
use xpallet_assets::ChainT;
use xpallet_gateway_common::traits::{ChannelBinding, Extractable};
use xpallet_support::{debug, error, info, str, warn, RUNTIME_TARGET};

// light-bitcoin
use btc_chain::Transaction;
use btc_keys::{Address, Network};
use btc_primitives::H256;
use btc_script::Script;

// use crate::traits::RelayTransaction;
#[cfg(feature = "std")]
use self::utils::trick_format_opreturn;
use self::utils::{
    equal_addr, inspect_address_from_transaction, is_key, parse_opreturn,
    parse_output_addr_with_networkid,
};
pub use self::validator::validate_transaction;
use crate::trustee::{get_last_trustee_address_pair, get_trustee_address_pair};
use crate::tx::utils::{addr2vecu8, ensure_identical};
use crate::types::{
    AccountInfo, BTCDepositCache, BTCTxResult, BTCTxState, DepositInfo, MetaTxType,
};
use crate::{
    AddressBinding, BoundAddressOf, Module, PendingDeposits, RawEvent, Trait, WithdrawalProposal,
};

pub fn process_tx<T: Trait>(
    tx: Transaction,
    prev: Option<Transaction>,
) -> result::Result<BTCTxState, DispatchError> {
    let meta_type = detect_transaction_type::<T>(&tx, prev.as_ref())?;
    // process
    let state = handle_tx::<T>(tx, meta_type);

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
///
/// when meet in `prev`, we would parse tx's inputs/outputs into Option<Address>
/// e.g
/// notice the relay tx only has first input
///        _________
///  addr |        | Some(addr)
///       |   tx   | Some(addr)
///       |________| None (OP_RETURN or something unknown)
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
    AccountId: Debug,
    F: Fn(&[u8]) -> Option<(AccountId, Option<Name>)>,
{
    let input_addr = prev.and_then(|prev_tx| {
        // parse input addr
        let outpoint = &tx.inputs[0].previous_output;
        inspect_address_from_transaction(prev_tx, outpoint, network)
    });

    // Withdrawal|TrusteeTransition|HotAndCold need input_addr to parse prev address
    if let Some(ref input_addr) = input_addr {
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
    AccountId: Debug,
    F: Fn(&[u8]) -> Option<(AccountId, Option<Name>)>,
{
    let (opreturn, deposit_value) =
        parse_deposit_outputs_impl(tx, hot_addr, network, handle_opreturn);
    if deposit_value >= min_deposit {
        if opreturn.is_none() && input_addr.is_none() {
            warn!("[detect_deposit_type]|receive a deposit tx but do not have valid opreturn & not have input addr|tx:{:?}", tx.hash());
            return MetaTxType::Irrelevance;
        }
        let info = DepositInfo {
            deposit_value,
            op_return: opreturn,
            input_addr: input_addr.map(Clone::clone),
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
    AccountId: Debug,
    F: Fn(&[u8]) -> Option<(AccountId, Option<Name>)>,
{
    let mut deposit_balance = 0;
    let mut account_info = None;
    let mut has_opreturn = false;
    let mut original: Vec<u8> = Default::default();
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
                        original = script.to_vec();
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

    native::debug!(
        target: RUNTIME_TARGET,
        "[parse_deposit_outputs_impl]|parse outputs|account_info:{:?}|balance:{:}|opreturn:{:}|",
        account_info,
        deposit_balance,
        trick_format_opreturn(&original)
    );
    (account_info, deposit_balance)
}

fn handle_tx<T: Trait>(tx: Transaction, meta_type: MetaTxType<T::AccountId>) -> BTCTxState {
    let tx_type = meta_type.ref_into();
    let result = match meta_type {
        MetaTxType::<_>::Deposit(deposit_info) => deposit::<T>(tx.hash(), deposit_info),
        MetaTxType::<_>::Withdrawal => withdraw::<T>(tx),
        _ => BTCTxResult::Success,
    };
    let state = BTCTxState { result, tx_type };
    state
}

fn deposit<T: Trait>(hash: H256, deposit_info: DepositInfo<T::AccountId>) -> BTCTxResult {
    let account_info = match deposit_info.op_return {
        Some((accountid, channel_name)) => {
            if let Some(addr) = deposit_info.input_addr {
                // remove old unbinding deposit info
                remove_pending_deposit::<T>(&addr, &accountid);
                // update or override binding info
                update_binding::<T>(&addr, &accountid);
            } else {
                // no input addr
                debug!("[deposit]|no input addr for this deposit tx, but has opreturn to get accountid|tx_hash:{:?}|who:{:?}", hash, accountid);
            }
            AccountInfo::<T::AccountId>::Account((accountid, channel_name))
        }
        None => {
            if let Some(addr) = deposit_info.input_addr {
                // no opreturn, use addr to get accountid
                match Module::<T>::address_binding(&addr) {
                    Some(accountid) => AccountInfo::Account((accountid, None)),
                    None => AccountInfo::Address(addr.clone()),
                }
            } else {
                error!("[deposit]|no input addr for this deposit tx, neither has opreturn to get accountid!|tx_hash:{:?}", hash);
                return BTCTxResult::Failed;
            }
        }
    };

    match account_info {
        AccountInfo::<_>::Account((accountid, channel_name)) => {
            T::Channel::update_binding(&<Module<T> as ChainT>::ASSET_ID, &accountid, channel_name);

            if let Err(_) = deposit_token::<T>(&accountid, deposit_info.deposit_value) {
                return BTCTxResult::Failed;
            }
            info!(
                "[deposit]|deposit success|who:{:?}|balance:{:}|tx_hash:{:}",
                accountid, deposit_info.deposit_value, hash
            );
        }
        AccountInfo::<_>::Address(addr) => {
            insert_pending_deposit::<T>(&addr, &hash, deposit_info.deposit_value);
            info!(
                "[deposit]|deposit into pending|addr:{:?}|balance:{:}|tx_hash:{:}",
                str!(addr2vecu8(&addr)),
                deposit_info.deposit_value,
                hash
            );
        }
    };
    BTCTxResult::Success
}

fn deposit_token<T: Trait>(who: &T::AccountId, balance: u64) -> DispatchResult {
    let id: AssetId = <Module<T> as ChainT>::ASSET_ID;

    let b: T::Balance = balance.saturated_into();
    let _ = <xpallet_gateway_records::Module<T>>::deposit(&who, &id, b).map_err(|e| {
        error!(
            "deposit error!, must use root to fix this error. reason:{:?}",
            e
        );
        e
    })?;
    Ok(())
}

fn update_binding<T: Trait>(address: &Address, who: &T::AccountId) {
    if let Some(accountid) = AddressBinding::<T>::get(&address) {
        if &accountid != who {
            debug!(
                "[apply_update_binding]|current binding need change|old:{:?}|new:{:?}",
                accountid, who
            );
            // old accountid is not equal to new accountid, means should change this addr bind to new account
            // remove this addr for old accounid's CrossChainBindOf
            BoundAddressOf::<T>::mutate(accountid, |addr_list| {
                addr_list.retain(|addr| addr != address);
            });
        }
    }
    // insert or override binding relationship
    BoundAddressOf::<T>::mutate(who, |addr_list| {
        let list = addr_list;
        if !list.contains(address) {
            list.push(address.clone());
        }
    });

    info!(
        "[apply_update_binding]|update binding|addr:{:?}|who:{:?}",
        str!(addr2vecu8(address)),
        who,
    );
    AddressBinding::<T>::insert(address, who.clone());
}

fn remove_pending_deposit<T: Trait>(input_address: &Address, who: &T::AccountId) {
    // notice this would delete this cache
    let records = PendingDeposits::take(input_address);
    for r in records {
        // ignore error
        let _ = deposit_token::<T>(who, r.balance);
        info!(
            "[remove_pending_deposit]|use pending info to re-deposit|who:{:?}|balance:{:}",
            who, r.balance
        );

        Module::<T>::deposit_event(RawEvent::DepositPending(
            who.clone(),
            r.balance.saturated_into(),
            r.txid,
            addr2vecu8(input_address),
        ));
    }
}

fn insert_pending_deposit<T: Trait>(input_address: &Address, txid: &H256, balance: u64) {
    let cache = BTCDepositCache {
        txid: txid.clone(),
        balance,
    };

    PendingDeposits::mutate(input_address, |list| {
        if !list.contains(&cache) {
            native::debug!(
                target: RUNTIME_TARGET,
                "[insert_pending_deposit]|Add pending deposit|address:{:?}|txhash:{:}|balance:{:}",
                str!(addr2vecu8(input_address)),
                txid,
                balance
            );
            list.push(cache);
        }
    });
}

fn withdraw<T: Trait>(tx: Transaction) -> BTCTxResult {
    if let Some(proposal) = WithdrawalProposal::<T>::take() {
        native::debug!(
            target: RUNTIME_TARGET,
            "[withdraw]|withdraw handle|proposal:{:?}|tx:{:?}",
            proposal,
            tx
        );
        match ensure_identical::<T>(&tx, &proposal.tx) {
            Ok(()) => {
                for number in proposal.withdrawal_id_list.iter() {
                    match xpallet_gateway_records::Module::<T>::finish_withdrawal(*number) {
                        Ok(_) => {
                            info!("[withdraw]|ID of withdrawal completion: {:}", *number);
                        }
                        Err(_e) => {
                            error!("[withdraw]|ID of withdrawal ERROR! {:}, reason:{:?}, please use root to fix it", *number, _e);
                        }
                    }
                }
                BTCTxResult::Success
            }
            Err(e) => {
                let tx_hash = proposal.tx.hash();
                error!("[withdraw]|Withdrawal failed, reason:{:?}, please use root to fix it|withdrawal idlist:{:?}|proposal id:{:?}|tx hash:{:}",
                       e, proposal.withdrawal_id_list, proposal.tx.hash(), tx.hash());
                WithdrawalProposal::<T>::put(proposal);

                Module::<T>::deposit_event(RawEvent::WithdrawalFatalErr(tx.hash(), tx_hash));
                // let _ = xfee_manager::Module::<T>::modify_switcher(
                //     xfee_manager::CallSwitcher::XBTC,
                //     true,
                // );
                BTCTxResult::Failed
            }
        }
    } else {
        error!("[withdraw]|Withdrawal failed, the proposal is EMPTY, but receive a withdrawal tx, please use root to fix it|tx hash:{:}", tx.hash());

        // no proposal, but find a withdraw tx, it's a fatal error in withdrawal
        Module::<T>::deposit_event(RawEvent::WithdrawalFatalErr(tx.hash(), Default::default()));

        // TODO use trait
        // let _ =
        //     xfee_manager::Module::<T>::modify_switcher(xfee_manager::CallSwitcher::XBTC, true);
        // do not return Err, mark this tx has been handled
        BTCTxResult::Failed
    }
}
