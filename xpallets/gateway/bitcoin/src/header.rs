// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{traits::UnixTime, StorageMap, StorageValue};
use sp_runtime::DispatchResult;
use sp_std::{
    cmp::{self, Ordering},
    convert::TryFrom,
    prelude::*,
};

use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    keys::Network as BtcNetwork,
    primitives::{hash_rev, Compact, H256, U256},
};

use xp_logging::{debug, error, info, warn};

use crate::{
    types::{BtcHeaderIndex, BtcHeaderInfo, BtcParams},
    ConfirmedIndex, Error, MainChain, Module, Trait,
};

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
    for cnt in 1..confirmations {
        if let Some(current_info) = Module::<T>::headers(&prev_hash) {
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
                "[look_back_confirmed_header] Can not find header ({:?}), current reverse count:{}",
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

pub fn update_confirmed_header<T: Trait>(header_info: &BtcHeaderInfo) -> Option<BtcHeaderIndex> {
    let (confirmed, chain) = look_back_confirmed_header::<T>(header_info);
    for index in chain {
        set_main_chain::<T>(index.height, index.hash);
    }
    confirmed.map(|index| {
        ConfirmedIndex::put(index);
        index
    })
}

fn set_main_chain<T: Trait>(height: u32, main_hash: H256) {
    let hashes = Module::<T>::block_hash_for(&height);
    if hashes.len() == 1 {
        // forked block header hash of this `height` does not exist
        MainChain::insert(&hashes[0], true);
        return;
    }
    for hash in hashes {
        if hash == main_hash {
            MainChain::insert(hash, true);
        } else {
            MainChain::remove(hash);
        }
    }
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

pub struct HeaderVerifier<'a> {
    pub work: HeaderWork<'a>,
    pub proof_of_work: HeaderProofOfWork<'a>,
    pub timestamp: HeaderTimestamp<'a>,
}

impl<'a> HeaderVerifier<'a> {
    pub fn new<T: Trait>(header_info: &'a BtcHeaderInfo) -> Self {
        let now = T::UnixTime::now();
        // if convert from u64 to u32 failed (unix timestamp should not be greater than u32::MAX),
        // ignore timestamp check, timestamp check are not important
        let current_time = u32::try_from(now.as_secs()).ok();

        Self {
            work: HeaderWork::new(header_info),
            proof_of_work: HeaderProofOfWork::new(&header_info.header),
            timestamp: HeaderTimestamp::new(&header_info.header, current_time),
        }
    }

    pub fn check<T: Trait>(&self) -> DispatchResult {
        let params: BtcParams = Module::<T>::params_info();
        let network_id: BtcNetwork = Module::<T>::network_id();
        if let BtcNetwork::Mainnet = network_id {
            self.work.check::<T>(&params)?;
        }
        self.proof_of_work.check::<T>(&params)?;
        // ignore this in benchmarks
        #[cfg(not(feature = "runtime-benchmarks"))]
        self.timestamp.check::<T>(&params)?;

        Ok(())
    }
}

pub struct HeaderWork<'a> {
    info: &'a BtcHeaderInfo,
}

impl<'a> HeaderWork<'a> {
    fn new(info: &'a BtcHeaderInfo) -> Self {
        HeaderWork { info }
    }

    fn check<T: Trait>(&self, params: &BtcParams) -> DispatchResult {
        let previous_header_hash = self.info.header.previous_header_hash;
        let work = work_required::<T>(previous_header_hash, self.info.height, params);
        if work != self.info.header.bits {
            error!(
                "[check_header_work] nBits do not match difficulty rules, work:{:?}, header bits:{:?}, height:{}",
                work, self.info.header.bits, self.info.height
            );
            return Err(Error::<T>::HeaderNBitsNotMatch.into());
        }
        Ok(())
    }
}

fn work_required<T: Trait>(parent_hash: H256, height: u32, params: &BtcParams) -> Compact {
    let max_bits = params.max_bits();
    if height == 0 {
        return max_bits;
    }

    let parent_header: BtcHeader = Module::<T>::headers(&parent_hash)
        .expect("pre header must exist here")
        .header;

    if is_retarget_height(height, params) {
        let new_work = work_required_retarget::<T>(parent_header, height, params);
        info!(
            "[work_required] Retarget new work required, height:{}, retargeting_interval:{}, new_work:{:?}",
            height, params.retargeting_interval(), new_work
        );
        return new_work;
    }
    debug!(
        "[work_required] Use old work required, old bits:{:?}",
        parent_header.bits
    );
    parent_header.bits
}

fn is_retarget_height(height: u32, params: &BtcParams) -> bool {
    height % params.retargeting_interval() == 0
}

/// Algorithm used for retargeting work every 2 weeks
fn work_required_retarget<T: Trait>(
    parent_header: BtcHeader,
    height: u32,
    params: &BtcParams,
) -> Compact {
    let retarget_num = height - params.retargeting_interval();

    // timestamp of parent block
    let last_timestamp = parent_header.time;
    // bits of last block
    let last_bits = parent_header.bits;

    let (genesis_header, genesis_height) = Module::<T>::genesis_info();
    let mut retarget_header = parent_header;
    if retarget_num < genesis_height {
        retarget_header = genesis_header;
    } else {
        let hash_list = Module::<T>::block_hash_for(&retarget_num);
        for h in hash_list {
            // look up in main chain
            if Module::<T>::main_chain(h) {
                let info = Module::<T>::headers(h).expect("block header must exist at here.");
                retarget_header = info.header;
                break;
            };
        }
    }

    // timestamp of block(height - RETARGETING_INTERVAL)
    let retarget_timestamp = retarget_header.time;

    let mut retarget: U256 = last_bits.into();
    let maximum: U256 = params.max_bits().into();

    retarget *= U256::from(retarget_timespan(
        retarget_timestamp,
        last_timestamp,
        params,
    ));
    retarget /= U256::from(params.target_timespan_seconds());

    debug!(
        "[work_required_retarget] retarget:{}, maximum:{:?}",
        retarget, maximum
    );

    if retarget > maximum {
        params.max_bits()
    } else {
        retarget.into()
    }
}

/// Returns constrained number of seconds since last retarget
fn retarget_timespan(retarget_timestamp: u32, last_timestamp: u32, params: &BtcParams) -> u32 {
    // TODO i64??
    // subtract unsigned 32 bit numbers in signed 64 bit space in
    // order to prevent underflow before applying the range constraint.
    let timespan = last_timestamp as i64 - i64::from(retarget_timestamp);
    range_constrain(
        timespan,
        i64::from(params.min_timespan()),
        i64::from(params.max_timespan()),
    ) as u32
}

fn range_constrain(value: i64, min: i64, max: i64) -> i64 {
    cmp::min(cmp::max(value, min), max)
}

pub struct HeaderProofOfWork<'a> {
    header: &'a BtcHeader,
}

impl<'a> HeaderProofOfWork<'a> {
    fn new(header: &'a BtcHeader) -> Self {
        Self { header }
    }

    fn check<T: Trait>(&self, params: &BtcParams) -> DispatchResult {
        if is_valid_proof_of_work(params.max_bits(), self.header.bits, self.header.hash()) {
            Ok(())
        } else {
            Err(Error::<T>::InvalidPoW.into())
        }
    }
}

fn is_valid_proof_of_work(max_work_bits: Compact, bits: Compact, hash: H256) -> bool {
    match (max_work_bits.to_u256(), bits.to_u256()) {
        (Ok(max), Ok(target)) => {
            let value = U256::from(hash_rev(hash).as_bytes());
            target <= max && value <= target
        }
        _ => false,
    }
}

pub struct HeaderTimestamp<'a> {
    header: &'a BtcHeader,
    current_time: Option<u32>,
}

impl<'a> HeaderTimestamp<'a> {
    fn new(header: &'a BtcHeader, current_time: Option<u32>) -> Self {
        Self {
            header,
            current_time,
        }
    }

    fn check<T: Trait>(&self, params: &BtcParams) -> DispatchResult {
        if let Some(current_time) = self.current_time {
            if self.header.time > current_time + params.block_max_future() {
                error!(
                    "[check_header_timestamp] Header time:{}, current time:{}, max_future{:?}",
                    self.header.time,
                    current_time,
                    params.block_max_future()
                );
                Err(Error::<T>::HeaderFuturisticTimestamp.into())
            } else {
                Ok(())
            }
        } else {
            // if get chain timestamp error, just ignore blockhead time check
            warn!(
                "[check_header_timestamp] Header:{:?}, get unix timestamp or calculate timestamp error, ignore it",
                hash_rev(self.header.hash())
            );
            Ok(())
        }
    }
}
