// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

mod header_proof;

// Substrate
use frame_support::{StorageMap, StorageValue};
use sp_runtime::DispatchResult;
use sp_std::{cmp::Ordering, prelude::*};

// ChainX
use xpallet_support::{error, info};

// light-bitcoin
use light_bitcoin::primitives::H256;

use crate::types::{BtcHeaderIndex, BtcHeaderInfo};
use crate::{ConfirmedIndex, Error, MainChain, Module, Trait};

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

/// todo move this issue to ChainX-org/ChainX repo
/// #issue 501 https://github.com/chainpool/ChainX/issues/501 would explain how to define
/// confirmation count
///      confirmed_height = now_height - (confirmations - 1)
///           |--- confirmations = 4 ---|
/// b(prev) - b(confirm)  -  b  -  b  - b
///           4              3     2    1 (confirmations)
///           97             98    99   100(height)
/// this function would pick the confirmed Index, and return the Index on the look back path
fn look_back_confirmed_header<T: Trait>(
    header_info: &BtcHeaderInfo,
) -> (Option<BtcHeaderIndex>, Vec<BtcHeaderIndex>) {
    let confirmations = Module::<T>::confirmation_number();
    let mut chain = Vec::with_capacity(confirmations as usize);
    let mut prev_hash = header_info.header.previous_header_hash;

    // put current header
    chain.push(BtcHeaderIndex {
        hash: header_info.header.hash(),
        height: header_info.height,
    });
    // e.g. when confirmations is 4, loop 3 times max
    for _i in 1..confirmations {
        if let Some(current_info) = Module::<T>::headers(&prev_hash) {
            chain.push(BtcHeaderIndex {
                hash: prev_hash,
                height: current_info.height,
            });
            prev_hash = current_info.header.previous_header_hash;
        } else {
            // if not find current header info, should be exceed genesis height, jump out of loop
            info!(
                "[update_confirmed_header]|not find for hash:{:?}, current reverse count:{:}",
                prev_hash, _i
            );
            break;
        }
    }
    let len = chain.len();
    if len == confirmations as usize {
        // confirmations must more than 0, thus, chain.last() must be some
        (chain.last().map(Clone::clone), chain)
    } else {
        (None, chain)
    }
}

pub fn update_confirmed_header<T: Trait>(header_info: &BtcHeaderInfo) -> Option<BtcHeaderIndex> {
    let (confirmed, chain) = look_back_confirmed_header::<T>(header_info);
    for index in chain {
        set_main_chain::<T>(index.height, &index.hash);
    }
    confirmed.map(|index| {
        ConfirmedIndex::put(index);
        index
    })

    // if let Some(index) = confirmed {
    //     // update confirmed index

    // } else {
    //     info!(
    //         "[update_confirmed_header]|not find prev header, use genesis instead|prev:{:?}",
    //         should_confirmed_hash
    //     );
    //     let info = Module::<T>::genesis_info();
    //     BtcHeaderIndex {
    //         hash: info.0.hash(),
    //         height: info.1,
    //     }
    // }
}

pub fn check_confirmed_header<T: Trait>(header_info: &BtcHeaderInfo) -> DispatchResult {
    let (confirmed, _) = look_back_confirmed_header::<T>(header_info);
    if let Some(current_confirmed) = ConfirmedIndex::get() {
        if let Some(now_confirmed) = confirmed {
            return match current_confirmed.height.cmp(&now_confirmed.height) {
                Ordering::Greater => {
                    // e.g:
                    //          current_confirmed
                    // b  ---------------- b  ------ b --- b --- b(best)
                    // |(now_confirmed)--- b  ------ b --- b(now)
                    // 99              100       101  102    103
                    // current_confirmed > now_ocnfirmed
                    Ok(())
                }
                Ordering::Equal => {
                    // e.g:
                    //current_confirmed
                    // b --------------- b  ------ b --- b(best)
                    // |(now_confirmed)- b  ------ b --- b(now)
                    // 99              100       101  102    103
                    // current_confirmed = now_confirmed
                    if current_confirmed.hash == now_confirmed.hash {
                        Ok(())
                    } else {
                        // e.g:
                        //
                        //  b --------- b(current_confirmed) b  ------ b --- b(best)
                        //  | --------- b(now_confirmed) --- b  ------ b --- b(now)
                        // 99              100       101  102    103
                        // current_confirmed = now_confirmed
                        Err(Error::<T>::AncientFork.into())
                    }
                }
                Ordering::Less => {
                    // normal should not happen, for call `check_confirmed_header` should under
                    // current <= best
                    error!("[check_confirmed_header]|should not happen, current confirmed is less than confirmed for this header|current:{:?}|now:{:?}", current_confirmed, now_confirmed);
                    Err(Error::<T>::AncientFork.into())
                }
            };
        }
    }
    // do not have confirmed yet.
    Ok(())
}

pub fn set_main_chain<T: Trait>(height: u32, main_hash: &H256) {
    let hashes = Module::<T>::block_hash_for(&height);
    if hashes.len() == 1 {
        MainChain::insert(&hashes[0], ());
        return;
    }
    for hash in hashes {
        if hash == *main_hash {
            MainChain::insert(&hash, ());
        } else {
            MainChain::remove(&hash);
        }
    }
}
