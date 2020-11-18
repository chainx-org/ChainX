// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

mod secp256k1_verifier;
pub mod validator;

use frame_support::{debug::native, dispatch::DispatchResult, StorageMap, StorageValue};
use sp_runtime::{traits::Zero, SaturatedConversion};
use sp_std::prelude::*;

use light_bitcoin::{
    chain::Transaction,
    keys::{Address, DisplayLayout, Network},
    primitives::{hash_rev, H256},
};

use chainx_primitives::AssetId;
use xp_gateway_bitcoin::{BtcDepositInfo, BtcTxMetaType, BtcTxTypeDetector};
use xp_gateway_common::AccountExtractor;
use xp_logging::{debug, error, info, warn};
use xpallet_assets::ChainT;
use xpallet_gateway_common::traits::{AddrBinding, ChannelBinding};
use xpallet_support::str;

pub use self::validator::validate_transaction;
use crate::{
    native,
    types::{AccountInfo, BtcAddress, BtcDepositCache, BtcTxResult, BtcTxState},
    BalanceOf, Error, Event, Module, PendingDeposits, Trait, WithdrawalProposal,
};

pub fn process_tx<T: Trait>(
    tx: Transaction,
    prev_tx: Option<Transaction>,
    network: Network,
    min_deposit: u64,
    current_trustee_pair: (Address, Address),
    previous_trustee_pair: Option<(Address, Address)>,
) -> BtcTxState {
    let btc_tx_detector = BtcTxTypeDetector::new(
        network,
        min_deposit,
        current_trustee_pair,
        previous_trustee_pair,
    );
    let meta_type = btc_tx_detector.detect_transaction_type::<T::AccountId, _>(
        &tx,
        prev_tx.as_ref(),
        T::AccountExtractor::extract_account,
    );

    let tx_type = meta_type.ref_into();
    let result = match meta_type {
        BtcTxMetaType::<_>::Deposit(deposit_info) => deposit::<T>(tx.hash(), deposit_info),
        BtcTxMetaType::<_>::Withdrawal => withdraw::<T>(tx),
        BtcTxMetaType::HotAndCold | BtcTxMetaType::TrusteeTransition => BtcTxResult::Success,
        // mark `Irrelevance` be `Failure` so that it could be replayed in the future
        BtcTxMetaType::<_>::Irrelevance => BtcTxResult::Failure,
    };

    BtcTxState { tx_type, result }
}

fn deposit<T: Trait>(txid: H256, deposit_info: BtcDepositInfo<T::AccountId>) -> BtcTxResult {
    let account_info = match (deposit_info.op_return, deposit_info.input_addr) {
        (Some((account, referral)), Some(input_addr)) => {
            let input_addr = addr2vecu8(&input_addr);
            // remove old unbinding deposit info
            remove_pending_deposit::<T>(&input_addr, &account);
            // update or override binding info
            T::AddrBinding::update_binding(Module::<T>::chain(), input_addr, account.clone());
            AccountInfo::<T::AccountId>::Account((account, referral))
        }
        (Some((account, referral)), None) => {
            // has opreturn but no input addr
            debug!(
                "[deposit] Deposit tx ({:?}) has no input addr, but has opreturn, who:{:?}",
                hash_rev(txid),
                account
            );
            AccountInfo::<T::AccountId>::Account((account, referral))
        }
        (None, Some(input_addr)) => {
            // no opreturn but have input addr, use input addr to get accountid
            let addr_bytes = addr2vecu8(&input_addr);
            match T::AddrBinding::get_binding(Module::<T>::chain(), addr_bytes) {
                Some(account) => AccountInfo::Account((account, None)),
                None => AccountInfo::Address(input_addr),
            }
        }
        (None, None) => {
            warn!(
                "[deposit] Process deposit tx ({:?}) but missing valid opreturn and input addr",
                hash_rev(txid)
            );
            return BtcTxResult::Failure;
        }
    };

    match account_info {
        AccountInfo::<_>::Account((account, referral)) => {
            T::Channel::update_binding(&<Module<T> as ChainT<_>>::ASSET_ID, &account, referral);
            match deposit_token::<T>(txid, &account, deposit_info.deposit_value) {
                Ok(_) => {
                    info!(
                        "[deposit] Deposit tx ({:?}) success, who:{:?}, balance:{}",
                        hash_rev(txid),
                        account,
                        deposit_info.deposit_value
                    );
                    BtcTxResult::Success
                }
                Err(_) => BtcTxResult::Failure,
            }
        }
        AccountInfo::<_>::Address(input_addr) => {
            insert_pending_deposit::<T>(&input_addr, txid, deposit_info.deposit_value);
            info!(
                "[deposit] Deposit tx ({:?}) into pending, addr:{:?}, balance:{}",
                hash_rev(txid),
                str!(addr2vecu8(&input_addr)),
                deposit_info.deposit_value
            );
            BtcTxResult::Success
        }
    }
}

fn deposit_token<T: Trait>(txid: H256, who: &T::AccountId, balance: u64) -> DispatchResult {
    let id: AssetId = <Module<T> as ChainT<_>>::ASSET_ID;

    let value: BalanceOf<T> = balance.saturated_into();
    match <xpallet_gateway_records::Module<T>>::deposit(&who, id, value) {
        Ok(()) => {
            Module::<T>::deposit_event(Event::<T>::Deposited(txid, who.clone(), value));
            Ok(())
        }
        Err(err) => {
            error!(
                "[deposit_token] Deposit error:{:?}, must use root to fix it",
                err
            );
            Err(err.into())
        }
    }
}

pub fn remove_pending_deposit<T: Trait>(input_address: &BtcAddress, who: &T::AccountId) {
    // notice this would delete this cache
    let records = PendingDeposits::take(input_address);
    for record in records {
        // ignore error
        let _ = deposit_token::<T>(record.txid, who, record.balance);
        info!(
            "[remove_pending_deposit] Use pending info to re-deposit, who:{:?}, balance:{}, cached_tx:{:?}",
            who, record.balance, record.txid,
        );

        Module::<T>::deposit_event(Event::<T>::PendingDepositRemoved(
            who.clone(),
            record.balance.saturated_into(),
            record.txid,
            input_address.clone(),
        ));
    }
}

fn insert_pending_deposit<T: Trait>(input_address: &Address, txid: H256, balance: u64) {
    let addr_bytes = addr2vecu8(input_address);

    let cache = BtcDepositCache { txid, balance };

    PendingDeposits::mutate(&addr_bytes, |list| {
        if !list.contains(&cache) {
            native::debug!(
                target: xp_logging::RUNTIME_TARGET,
                "[insert_pending_deposit] Add pending deposit, address:{:?}, txhash:{:?}, balance:{}",
                str!(addr_bytes),
                txid,
                balance
            );
            list.push(cache);

            Module::<T>::deposit_event(Event::<T>::UnclaimedDeposit(txid, addr_bytes.clone()));
        }
    });
}

fn withdraw<T: Trait>(tx: Transaction) -> BtcTxResult {
    if let Some(proposal) = WithdrawalProposal::<T>::take() {
        native::debug!(
            target: xp_logging::RUNTIME_TARGET,
            "[withdraw] Withdraw tx {:?}, proposal:{:?}",
            proposal,
            tx
        );
        let proposal_hash = proposal.tx.hash();
        let tx_hash = tx.hash();

        if proposal_hash == tx_hash {
            let mut total = BalanceOf::<T>::zero();
            for number in proposal.withdrawal_id_list.iter() {
                // just for event record
                let withdraw_balance =
                    xpallet_gateway_records::Module::<T>::pending_withdrawals(number)
                        .map(|record| record.balance())
                        .unwrap_or(BalanceOf::<T>::zero());
                total += withdraw_balance;

                match xpallet_gateway_records::Module::<T>::finish_withdrawal(*number, None) {
                    Ok(_) => {
                        info!("[withdraw] Withdrawal ({}) completion", *number);
                    }
                    Err(err) => {
                        error!(
                            "[withdraw] Withdrawal ({}) error:{:?}, must use root to fix it",
                            *number, err
                        );
                    }
                }
            }

            let btc_withdrawal_fee = Module::<T>::btc_withdrawal_fee();
            // real withdraw value would reduce withdraw_fee
            total -=
                (proposal.withdrawal_id_list.len() as u64 * btc_withdrawal_fee).saturated_into();
            Module::<T>::deposit_event(Event::<T>::Withdrawn(
                tx_hash,
                proposal.withdrawal_id_list,
                total,
            ));
            BtcTxResult::Success
        } else {
            error!(
                "[withdraw] Withdraw error: mismatch (tx_hash:{:?}, proposal_hash:{:?}), id_list:{:?}, must use root to fix it",
                tx_hash, proposal_hash, proposal.withdrawal_id_list
            );
            // re-store proposal into storage.
            WithdrawalProposal::<T>::put(proposal);

            Module::<T>::deposit_event(Event::<T>::WithdrawalFatalErr(proposal_hash, tx_hash));
            BtcTxResult::Failure
        }
    } else {
        error!(
            "[withdraw] Withdrawal error: proposal is EMPTY (tx_hash:{:?}), but receive a withdrawal tx, must use root to fix it",
            tx.hash()
        );
        // no proposal, but find a withdraw tx, it's a fatal error in withdrawal
        Module::<T>::deposit_event(Event::<T>::WithdrawalFatalErr(
            tx.hash(),
            Default::default(),
        ));

        BtcTxResult::Failure
    }
}

/// Returns Ok if `tx1` and `tx2` are the same transaction.
pub fn ensure_identical<T: Trait>(tx1: &Transaction, tx2: &Transaction) -> DispatchResult {
    if tx1.version == tx2.version
        && tx1.outputs == tx2.outputs
        && tx1.lock_time == tx2.lock_time
        && tx1.inputs.len() == tx2.inputs.len()
    {
        for i in 0..tx1.inputs.len() {
            if tx1.inputs[i].previous_output != tx2.inputs[i].previous_output
                || tx1.inputs[i].sequence != tx2.inputs[i].sequence
            {
                native!(
                    error,
                    "[ensure_identical] Tx1 is different to Tx2, tx1:{:?}, tx2:{:?}",
                    tx1,
                    tx2
                );
                return Err(Error::<T>::MismatchedTx.into());
            }
        }
        return Ok(());
    }
    native!(
        error,
        "The transaction text does not match the original text to be signed",
    );
    Err(Error::<T>::MismatchedTx.into())
}

#[inline]
pub fn addr2vecu8(addr: &Address) -> Vec<u8> {
    bs58::encode(&*addr.layout()).into_vec()
}
