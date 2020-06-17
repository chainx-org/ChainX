// Copyright 2018-2019 Chainpool.

// Substrate
use sp_std::prelude::Vec;

// CHainX
use xassets::{Chain, ChainT};
use xrecords::{self, HeightOrTime, RecordInfo, TxState};
use xsupport::error;

use super::tx::handler::parse_deposit_outputs;
use super::tx::utils::{addr2vecu8, ensure_identical};
use super::types::{TxType, VoteResult};
use super::{Module, Trait};

impl<T: Trait> Module<T> {
    pub fn withdrawal_list() -> Vec<RecordInfo<T::AccountId, T::Balance, T::BlockNumber, T::Moment>>
    {
        let mut records = xrecords::Module::<T>::withdrawal_applications(Chain::Bitcoin)
            .into_iter()
            .map(|appl| RecordInfo {
                who: appl.applicant(),
                token: appl.token(),
                balance: appl.balance(),
                txid: Vec::new(),
                addr: appl.addr(), // for btc, it's bas58 addr
                ext: appl.ext(),
                height_or_time: HeightOrTime::<T::BlockNumber, T::Moment>::Height(appl.height()),
                withdrawal_id: appl.id(), // only for withdrawal
                state: TxState::Applying,
                application_state: Some(appl.state()),
            })
            .collect::<Vec<_>>();

        match Self::withdrawal_proposal() {
            None => {
                // no proposal, all records is under applying
            }
            Some(proposal) => {
                // find proposal txhash
                let confirmations = Module::<T>::confirmation_number();
                let mut current_hash = Self::best_index();
                let mut tx_hash: Vec<u8> = Default::default();
                let mut tx_confirmed = 1;
                // not include confirmed block, when confirmations = 6, it's 0..5 => [0,1,2,3,4]
                // b(100)(confirmed) - b(101) - b(102) - b(103) - b(104) - b(105)(best)
                //                                                         current 1
                //                                              current 2
                //                                    current 3
                //                           current 4
                //                  current 5
                for confirmed in 1_u32..confirmations {
                    if let Some(info) = Module::<T>::block_header_for(current_hash) {
                        // lookup withdrawal tx in current header
                        for txid in info.txid_list {
                            if let Some(tx_info) = Self::tx_for(&txid) {
                                if tx_info.tx_type == TxType::Withdrawal {
                                    if let Some(proposal) = Self::withdrawal_proposal() {
                                        // only this tx total equal to proposal, choose this txhash
                                        if let Ok(()) =
                                            ensure_identical(&tx_info.raw_tx, &proposal.tx)
                                        {
                                            tx_hash = tx_info.raw_tx.hash().as_ref().to_vec();
                                            tx_confirmed = confirmed;
                                        }
                                    }
                                }
                            }
                        }
                        current_hash = info.header.previous_header_hash
                    } else {
                        error!(
                            "[withdrawal_list] error!, could not find block for this hash[{:}]!",
                            current_hash
                        );
                        break;
                    }
                }
                for record in records.iter_mut() {
                    if proposal
                        .withdrawal_id_list
                        .iter()
                        .any(|id| *id == record.withdrawal_id)
                    {
                        // in proposal, change state , not in proposal, state is Applying
                        record.state = match proposal.sig_state {
                            VoteResult::Unfinish => TxState::Signing,
                            VoteResult::Finish => {
                                if tx_hash.len() != 0 {
                                    record.txid = tx_hash.clone();
                                    TxState::Confirming(tx_confirmed, confirmations)
                                } else {
                                    TxState::Broadcasting
                                }
                            }
                        };
                    }
                }
            }
        }

        records
    }

    pub fn deposit_list() -> Vec<RecordInfo<T::AccountId, T::Balance, T::BlockNumber, T::Moment>> {
        let mut records = Vec::new();

        // find proposal txhash
        let confirmations = Module::<T>::confirmation_number();
        // not include confirmed block, when confirmations = 6, it's 0..5 => [0,1,2,3,4]
        // b(100)(confirmed) - b(101) - b(102) - b(103) - b(104) - b(105)(best)
        //                                                         current 0
        //                                              current 1
        //                                    current 2
        //                           current 3
        //                  current 4
        let mut current_hash = Self::best_index();
        for index in 0..(confirmations - 1) {
            if let Some(info) = Module::<T>::block_header_for(current_hash) {
                for txid in info.txid_list {
                    if let Some(tx_info) = Self::tx_for(&txid) {
                        if tx_info.tx_type == TxType::Deposit {
                            let timestamp = info.header.time;
                            // for 0 is 1 confirmed(s), not zero
                            let state = TxState::Confirming(index + 1, confirmations);

                            let r = match parse_deposit_outputs::<T>(&tx_info.raw_tx) {
                                Ok(r) => r,
                                Err(_e) => {
                                    error!("[deposit_list] error!, parse deposit outputs error, info: {:?}", _e);
                                    return Vec::new();
                                }
                            };

                            let (account_info, balance, ext) = r;

                            let tx_hash = tx_info.raw_tx.hash();
                            let info =
                                RecordInfo::<T::AccountId, T::Balance, T::BlockNumber, T::Moment> {
                                    who: account_info.map(|(a, _)| a).unwrap_or_default(),
                                    token: Self::TOKEN.to_vec(),
                                    balance: balance.into(),
                                    txid: tx_hash.as_ref().to_vec(),
                                    addr: Self::input_addr_for(tx_hash)
                                        .map(|addr| addr2vecu8(&addr))
                                        .unwrap_or_default(),
                                    ext: ext.unwrap_or_default(), // op return
                                    height_or_time:
                                        HeightOrTime::<T::BlockNumber, T::Moment>::Timestamp(
                                            timestamp.into(),
                                        ),
                                    withdrawal_id: 0, // only for withdrawal
                                    state,
                                    application_state: None,
                                };
                            records.push(info);
                        }
                    }
                }
                current_hash = info.header.previous_header_hash
            } else {
                error!(
                    "[deposit_list] error!, could not find block for this hash[{:}]!",
                    current_hash
                );
                return Vec::new();
            }
        }

        records
    }
}
