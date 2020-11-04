// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

// Substrate
use frame_support::{dispatch::DispatchResult, traits::UnixTime};
use sp_std::{cmp, convert::TryFrom, result};

// ChainX
use xp_logging::{debug, error, info, warn};

// light-bitcoin
use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    keys::Network,
    primitives::{Compact, H256, U256},
};

use super::ChainErr;
use crate::types::{BtcHeaderInfo, BtcParams};
use crate::{Error, Module, Trait};

pub struct HeaderVerifier<'a> {
    pub work: HeaderWork<'a>,
    pub proof_of_work: HeaderProofOfWork<'a>,
    pub timestamp: HeaderTimestamp<'a>,
}

impl<'a> HeaderVerifier<'a> {
    pub fn new<T: Trait>(header_info: &'a BtcHeaderInfo) -> result::Result<Self, ChainErr> {
        let current = T::UnixTime::now();
        // if convert from u64 to u32 failed, ignore timestamp check
        // timestamp check are not important
        let current_time = u32::try_from(current.as_secs()).ok();

        Ok(Self {
            work: HeaderWork::new(header_info),
            proof_of_work: HeaderProofOfWork::new(&header_info.header),
            timestamp: HeaderTimestamp::new(&header_info.header, current_time),
        })
    }

    pub fn check<T: Trait>(&self) -> DispatchResult {
        let params: BtcParams = Module::<T>::params_info();
        let network_id: Network = Module::<T>::network_id();
        if let Network::Mainnet = network_id {
            self.work.check::<T>(&params)?;
        }
        self.proof_of_work.check::<T>(&params)?;
        // ignore this in benchmarks
        #[cfg(not(feature = "runtime-benchmarks"))]
        {
            self.timestamp.check::<T>(&params)?;
        }

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

    fn check<T: Trait>(&self, p: &BtcParams) -> DispatchResult {
        let previous_header_hash = self.info.header.previous_header_hash;
        let work = work_required::<T>(previous_header_hash, self.info.height, p);
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

pub fn work_required<T: Trait>(parent_hash: H256, height: u32, params: &BtcParams) -> Compact {
    let max_bits = params.max_bits();
    if height == 0 {
        return max_bits;
    }

    let parent_header: BtcHeader = Module::<T>::headers(&parent_hash).unwrap().header;

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

pub fn is_retarget_height(height: u32, p: &BtcParams) -> bool {
    height % p.retargeting_interval() == 0
}

/// Algorithm used for retargeting work every 2 weeks
pub fn work_required_retarget<T: Trait>(
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
pub fn retarget_timespan(retarget_timestamp: u32, last_timestamp: u32, p: &BtcParams) -> u32 {
    // TODO i64??
    // subtract unsigned 32 bit numbers in signed 64 bit space in
    // order to prevent underflow before applying the range constraint.
    let timespan = last_timestamp as i64 - i64::from(retarget_timestamp);
    range_constrain(
        timespan,
        i64::from(p.min_timespan()),
        i64::from(p.max_timespan()),
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

    fn check<T: Trait>(&self, p: &BtcParams) -> DispatchResult {
        if is_valid_proof_of_work(p.max_bits(), self.header.bits, &self.header.hash()) {
            Ok(())
        } else {
            Err(Error::<T>::InvalidPoW.into())
        }
    }
}

fn reverse_hash256(hash: &H256) -> H256 {
    let mut res: H256 = H256::from_slice(hash.as_bytes());
    let bytes = res.as_bytes_mut();
    bytes.reverse();
    res
}

pub fn is_valid_proof_of_work(max_work_bits: Compact, bits: Compact, hash: &H256) -> bool {
    let maximum = match max_work_bits.to_u256() {
        Ok(max) => max,
        _err => return false,
    };

    let target = match bits.to_u256() {
        Ok(target) => target,
        _err => return false,
    };

    let value = U256::from(reverse_hash256(hash).as_bytes());
    target <= maximum && value <= target
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

    fn check<T: Trait>(&self, p: &BtcParams) -> DispatchResult {
        if let Some(current_time) = self.current_time {
            if self.header.time > current_time + p.block_max_future() {
                error!(
                    "[check_header_timestamp] Header time:{}, current time:{}, max_future{:?}",
                    self.header.time,
                    current_time,
                    p.block_max_future()
                );
                Err(Error::<T>::HeaderFuturisticTimestamp.into())
            } else {
                Ok(())
            }
        } else {
            // if get chain timestamp error, just ignore blockhead time check
            warn!(
                "[check_header_timestamp] Header:{:?}, get unix timestamp error, ignore it",
                self.header.hash()
            );
            Ok(())
        }
    }
}
