// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

mod header_proof;

use frame_support::log::{error, info};
use sp_runtime::DispatchResult;
use sp_std::{cmp::Ordering, prelude::*};

use light_bitcoin::primitives::{hash_rev, H256};

use crate::{
    types::{BtcHeaderIndex, BtcHeaderInfo},
    Config, ConfirmedIndex, Error, MainChain, Pallet,
};

pub use self::header_proof::HeaderVerifier;

/// Look back the headers to pick the confirmed index,
/// return the header indexes on the look back path.
///
/// The definition of block confirmation count:
/// confirmed_height = now_height - (confirmations - 1)
///           |--- confirmations = 4 ---|
/// b(prev) - b(confirm)  -  b  -  b  - b
///           4              3     2    1       (confirmations)
///           97             98    99   100     (height)
///
fn look_back_confirmed_header<T: Config>(
    header_info: &BtcHeaderInfo,
) -> (Option<BtcHeaderIndex>, Vec<BtcHeaderIndex>) {
    let confirmations = Pallet::<T>::confirmation_number();
    let mut chain = Vec::with_capacity(confirmations as usize);
    let mut prev_hash = header_info.header.previous_header_hash;

    // put current header
    chain.push(BtcHeaderIndex {
        hash: header_info.header.hash(),
        height: header_info.height,
    });
    // e.g. when confirmations is 4, loop 3 times max
    for cnt in 1..confirmations {
        if let Some(current_info) = Pallet::<T>::headers(&prev_hash) {
            chain.push(BtcHeaderIndex {
                hash: prev_hash,
                height: current_info.height,
            });
            prev_hash = current_info.header.previous_header_hash;
        } else {
            // if cannot find current header info, should be exceed genesis height, jump out of loop
            // e.g. want to get the previous header of #98, but genesis height is 98,
            // obviously, we cannot find the header of #97.
            info!(
                target: "runtime::bitcoin",
                "[update_confirmed_header] Can not find header ({:?}), current reverse count:{}",
                hash_rev(prev_hash),
                cnt
            );
            break;
        }
    }
    if chain.len() == confirmations as usize {
        // confirmations must more than 0, thus, chain.last() must be some
        (chain.last().cloned(), chain)
    } else {
        (None, chain)
    }
}

pub fn update_confirmed_header<T: Config>(header_info: &BtcHeaderInfo) -> Option<BtcHeaderIndex> {
    let (confirmed, chain) = look_back_confirmed_header::<T>(header_info);
    for index in chain {
        set_main_chain::<T>(index.height, index.hash);
    }
    confirmed.map(|index| {
        ConfirmedIndex::<T>::put(index);
        index
    })
}

fn set_main_chain<T: Config>(height: u32, main_hash: H256) {
    let hashes = Pallet::<T>::block_hash_for(&height);
    if hashes.len() == 1 {
        MainChain::<T>::insert(&hashes[0], true);
        return;
    }
    for hash in hashes {
        if hash == main_hash {
            MainChain::<T>::insert(&hash, true);
        } else {
            MainChain::<T>::remove(&hash);
        }
    }
}

pub fn check_confirmed_header<T: Config>(header_info: &BtcHeaderInfo) -> DispatchResult {
    let (confirmed, _) = look_back_confirmed_header::<T>(header_info);
    if let Some(current_confirmed) = ConfirmedIndex::<T>::get() {
        if let Some(now_confirmed) = confirmed {
            return match current_confirmed.height.cmp(&now_confirmed.height) {
                Ordering::Greater => {
                    // e.g:
                    //          current_confirmed
                    // b  ---------------- b  ------ b --- b --- b(best)
                    // |(now_confirmed)--- b  ------ b --- b(now)
                    // 99              100       101  102    103
                    // current_confirmed > now_confirmed
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
                    error!(
                        "[check_confirmed_header] Should not happen, current confirmed is less than confirmed for this header, \
                        current:{:?}, now:{:?}", current_confirmed, now_confirmed
                    );
                    Err(Error::<T>::AncientFork.into())
                }
            };
        }
    }
    // do not have confirmed yet.
    Ok(())
}
