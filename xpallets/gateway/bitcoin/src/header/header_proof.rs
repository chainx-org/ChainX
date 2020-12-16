// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{dispatch::DispatchResult, traits::UnixTime};
use sp_runtime::RuntimeDebug;
use sp_std::{cmp, convert::TryFrom};

use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    keys::Network,
    primitives::{hash_rev, Compact, H256, U256},
};

use xp_logging::{debug, error, info, warn};

use crate::types::{BtcHeaderInfo, BtcParams};
use crate::{Error, Module, Trait};

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
        let network_id: Network = Module::<T>::network_id();
        if let Network::Mainnet = network_id {
            self.work.check::<T>(&params)?;
        }
        self.proof_of_work.check::<T>(&params)?;
        // ignore this in benchmarks
        #[cfg(not(feature = "runtime-benchmarks"))]
        self.timestamp.check::<T>(&params)?;

        Ok(())
    }
}

#[derive(RuntimeDebug)]
enum RequiredWork {
    Value(Compact),
    NotCheck,
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
        match work {
            RequiredWork::Value(work) => {
                if work != self.info.header.bits {
                    error!(
                        "[check_header_work] nBits do not match difficulty rules, work:{:?}, header bits:{:?}, height:{}",
                        work, self.info.header.bits, self.info.height
                    );
                    return Err(Error::<T>::HeaderNBitsNotMatch.into());
                }
                Ok(())
            }
            RequiredWork::NotCheck => Ok(()),
        }
    }
}

pub fn work_required<T: Trait>(parent_hash: H256, height: u32, params: &BtcParams) -> RequiredWork {
    let max_bits = params.max_bits();
    if height == 0 {
        return RequiredWork::Value(max_bits);
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
    RequiredWork::Value(parent_header.bits)
}

fn is_retarget_height(height: u32, params: &BtcParams) -> bool {
    height % params.retargeting_interval() == 0
}

/// Algorithm used for retargeting work every 2 weeks
fn work_required_retarget<T: Trait>(
    parent_header: BtcHeader,
    height: u32,
    params: &BtcParams,
) -> RequiredWork {
    let retarget_num = height - params.retargeting_interval();

    // timestamp of parent block
    let last_timestamp = parent_header.time;
    // bits of last block
    let last_bits = parent_header.bits;

    let (genesis_header, genesis_height) = Module::<T>::genesis_info();
    let mut retarget_header = parent_header;
    if retarget_num < genesis_height {
        // retarget_header = genesis_header;
        return RequiredWork::NotCheck;
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

    RequiredWork::Value(if retarget > maximum {
        params.max_bits()
    } else {
        retarget.into()
    })
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
        (Ok(maximum), Ok(target)) => {
            let value = U256::from(hash_rev(hash).as_bytes());
            target <= maximum && value <= target
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

    #[allow(unused)]
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
                "[check_header_timestamp] Header:{:?}, get unix timestamp error, ignore it",
                hash_rev(self.header.hash())
            );
            Ok(())
        }
    }
}
