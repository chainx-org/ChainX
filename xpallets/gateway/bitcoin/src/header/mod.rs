// Copyright 2018-2019 Chainpool.

mod header_proof;

// Substrate
use frame_support::{dispatch::DispatchResult, StorageMap, StorageValue};
use sp_std::result;

// ChainX
use xpallet_support::{debug, error, info};

// light-bitcoin
use btc_chain::BlockHeader as BTCHeader;
use btc_primitives::H256;

use crate::types::{BTCHeaderIndex, BTCHeaderInfo};
use crate::{BTCHeaderFor, BlockHashFor, ConfirmedHeader, Error, MainChain, Module, Trait};

pub use self::header_proof::HeaderVerifier;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ChainErr {
    /// Not Found
    NotFound,
    /// Ancient fork
    AncientFork,
}

impl ChainErr {
    pub fn to_err<T: Trait>(self) -> Error<T> {
        self.into()
    }
}

impl<T: Trait> From<ChainErr> for Error<T> {
    fn from(err: ChainErr) -> Self {
        match err {
            ChainErr::NotFound => Error::<T>::HeaderNotFound,
            ChainErr::AncientFork => Error::<T>::HeaderAncientFork,
        }
    }
}
/*
pub fn remove_unused_headers<T: Trait>(header_info: &BTCHeaderInfo) {
    //delete old header info
    let reserved = Module::<T>::reserved_block();
    if header_info.height > reserved {
        let del = header_info.height - reserved;
        let v = Module::<T>::block_hash_for(&del);
        // remove all block for this height
        for h in v.iter() {
            // if let Some(header_info) = Module::<T>::btc_header_for(h) {
                // // remove related tx for this block
                // for txid in header_info.txid_list.iter() {
                //     // TODO
                //     // remove_unused_tx::<T>(txid);
                // }
            // }

            BTCHeaderFor::remove(h);
            debug!(
                "[remove_unused_headers]|remove old header|height:{:}|hash:{:?}",
                del, h
            );
        }
        BlockHashFor::remove(&del);
    }
}
*/
///      confirmed = best_height - (confirmations - 1)
///           |--------- confirmations = 6 ------------|
/// b(prev) - b(confirm) - b - b - b - b - b(best_index)
/// #issue 501 https://github.com/chainpool/ChainX/issues/501
pub fn update_confirmed_header<T: Trait>(header_info: &BTCHeaderInfo) -> BTCHeaderIndex {
    // update confirmd status
    let confirmations = Module::<T>::confirmation_number();
    let mut prev_hash = header_info.header.previous_header_hash.clone();
    // start from prev, thus start from 1,when confirmations = 6, it's 1..5 => [1,2,3,4]
    // b(100)(confirmed) - b(101)(need_confirmed) - b(102) - b(103) - b(104) - b(105)(best) - b(106)(current)
    //                                                                           prev        current 0
    //                                                                 prev     current 1 (start loop from this)
    //                                                       prev     current 2
    //                                              prev     current 3
    //                                  prev     current 4
    set_main_chain::<T>(header_info.height, &header_info.header.hash());
    for _i in 1..(confirmations - 1) {
        if let Some(current_info) = Module::<T>::btc_header_for(&prev_hash) {
            set_main_chain::<T>(current_info.height, &prev_hash);
            prev_hash = current_info.header.previous_header_hash;
        } else {
            // if not find current header info, jump out of loop
            info!(
                "[update_confirmed_header]|not find for hash:{:?}, current reverse count:{:}",
                prev_hash, _i
            );
            break;
        }
    }

    if let Some(mut info) = Module::<T>::btc_header_for(&prev_hash) {
        let index = BTCHeaderIndex {
            hash: prev_hash,
            height: info.height,
        };
        // update confirmed index
        ConfirmedHeader::put(index);
        index
    } else {
        // no not have prev hash in storage, return genesis header info
        info!(
            "[update_confirmed_header]|not find prev header, use genesis instead|prev:{:?}",
            prev_hash
        );
        let info = Module::<T>::genesis_info();
        BTCHeaderIndex {
            hash: info.0.hash(),
            height: info.1,
        }
    }
}
/*
fn handle_confirmed_block<T: Trait>(confirmed_header: &BTCHeaderInfo) {
    debug!(
        "[handle_confirmed_block]|Confirmed: height:{:}|hash:{:}",
        confirmed_header.height as u64,
        confirmed_header.header.hash(),
    );
    for _txid in confirmed_header.txid_list.iter() {
        // deposit & withdraw
        // TODO
        // match handle_tx::<T>(txid) {
        //     Err(_e) => {
        //         error!(
        //             "[handle_confirmed_block]|Handle tx failed, the error info:{:}|tx_hash:{:}",
        //             _e, txid,
        //         );
        //     }
        //     Ok(()) => (),
        // }
    }
}
*/
/*
/// not include confirmed block, when confirmations = 6, it's 0..5 => [0,1,2,3,4]
/// b(100)(confirmed) - b(101) - b(102) - b(103) - b(104) - b(105)(best)
///                                                         current 0
///                                              current 1
///                                    current 2
///                           current 3
///       prev        current 4
pub fn find_confirmed_block<T: Trait>(current: &H256) -> BTCHeaderInfo {
    let confirmations = Module::<T>::confirmation_number();
    let mut current_hash = current.clone();
    for _ in 0..(confirmations - 1) {
        if let Some(info) = Module::<T>::btc_header_for(current_hash) {
            if info.confirmed == true {
                return info;
            }

            current_hash = info.header.previous_header_hash
        } else {
            break;
        }
    }

    if let Some(info) = Module::<T>::btc_header_for(current_hash) {
        info
    } else {
        let (header, _) = Module::<T>::genesis_info();
        Module::<T>::btc_header_for(header.hash()).expect("genesis hash must exist!")
    }
}
*/

pub fn set_main_chain<T: Trait>(height: u32, main_hash: &H256) {
    let hashes = Module::<T>::block_hash_for(&height);
    if hashes.len() == 1 {
        MainChain::insert(&hashes[0], ());
        return;
    }
    for hash in hashes {
        if hash == *main_hash {
            // TODO detect not exist state
            MainChain::insert(&hash, ());
        } else {
            MainChain::remove(&hash);
        }
    }
}
