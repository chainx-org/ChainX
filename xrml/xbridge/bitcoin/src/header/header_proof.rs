// Copyright 2018-2019 Chainpool.

// Substrate
use primitives::traits::As;
use rstd::{cmp, result};
use support::dispatch::Result;

// ChainX
use xsupport::{debug, ensure_with_errorlog, error, info};

// light-bitcoin
use btc_chain::BlockHeader;
use btc_primitives::{Compact, H256, U256};

use super::ChainErr;
use crate::types::Params;
use crate::{Module, Trait};

pub struct HeaderVerifier<'a> {
    pub work: HeaderWork<'a>,
    pub proof_of_work: HeaderProofOfWork<'a>,
    pub timestamp: HeaderTimestamp<'a>,
}

impl<'a> HeaderVerifier<'a> {
    pub fn new<T: Trait>(header: &'a BlockHeader, height: u32) -> result::Result<Self, ChainErr> {
        let now: T::Moment = timestamp::Module::<T>::now();
        let current_time: u32 = now.as_() as u32;

        Ok(HeaderVerifier {
            work: HeaderWork::new(header, height),
            proof_of_work: HeaderProofOfWork::new(header),
            timestamp: HeaderTimestamp::new(header, current_time),
        })
    }

    pub fn check<T: Trait>(&self) -> Result {
        let params: Params = Module::<T>::params_info();
        let network_id: u32 = Module::<T>::network_id();
        if network_id == 0 {
            self.work.check::<T>(&params)?;
        }
        self.proof_of_work.check(&params)?;
        self.timestamp.check(&params)?;
        Ok(())
    }
}

pub struct HeaderWork<'a> {
    header: &'a BlockHeader,
    height: u32,
}

impl<'a> HeaderWork<'a> {
    fn new(header: &'a BlockHeader, height: u32) -> Self {
        HeaderWork { header, height }
    }

    fn check<T: Trait>(&self, p: &Params) -> Result {
        let previous_header_hash = self.header.previous_header_hash.clone();
        let work = work_required::<T>(previous_header_hash, self.height, p);
        ensure_with_errorlog!(
            work == self.header.bits,
            "nBits do not match difficulty rules",
            "work{:?}|header bits:{:?}|height:{:}",
            work,
            self.header.bits,
            self.height
        );
        Ok(())
    }
}

pub fn work_required<T: Trait>(parent_hash: H256, height: u32, params: &Params) -> Compact {
    let max_bits = params.max_bits().into();
    if height == 0 {
        return max_bits;
    }

    let parent_header: BlockHeader = Module::<T>::block_header_for(&parent_hash).unwrap().header;

    if is_retarget_height(height, params) {
        let new_work = work_required_retarget::<T>(parent_header, height, params);
        info!("[work_required]|retaget new work required|height:{:}|retargeting_interval:{:}|new_work:{:?}", height, params.retargeting_interval(), new_work);
        return new_work;
    }
    debug!(
        "[work_required]|use old work requrie|old bits:{:?}",
        parent_header.bits
    );
    parent_header.bits
}

pub fn is_retarget_height(height: u32, p: &Params) -> bool {
    height % p.retargeting_interval() == 0
}

/// Algorithm used for retargeting work every 2 weeks
pub fn work_required_retarget<T: Trait>(
    parent_header: BlockHeader,
    height: u32,
    params: &Params,
) -> Compact {
    let retarget_num = height - params.retargeting_interval();

    let (genesis_header, genesis_num) = Module::<T>::genesis_info();
    let mut retarget_header = parent_header.clone();
    if retarget_num < genesis_num {
        retarget_header = genesis_header;
    } else {
        let hash_list = Module::<T>::block_hash_for(&retarget_num);
        for h in hash_list {
            // look up in main chain
            if let Some(info) = Module::<T>::block_header_for(h) {
                if info.confirmed == true {
                    retarget_header = info.header;
                    break;
                }
            };
        }
    }

    // timestamp of block(height - RETARGETING_INTERVAL)
    let retarget_timestamp = retarget_header.time;
    // timestamp of parent block
    let last_timestamp = parent_header.time;
    // bits of last block
    let last_bits = parent_header.bits;

    let mut retarget: U256 = last_bits.into();
    let maximum: U256 = params.max_bits().into();

    retarget = retarget
        * U256::from(retarget_timespan(
            retarget_timestamp,
            last_timestamp,
            params,
        ));
    retarget = retarget / U256::from(params.target_timespan_seconds());

    debug!(
        "[work_required_retarget]|retarget:{:}|maximum:{:?}",
        retarget, maximum
    );

    if retarget > maximum {
        params.max_bits()
    } else {
        retarget.into()
    }
}

/// Returns constrained number of seconds since last retarget
pub fn retarget_timespan(retarget_timestamp: u32, last_timestamp: u32, p: &Params) -> u32 {
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
    header: &'a BlockHeader,
}

impl<'a> HeaderProofOfWork<'a> {
    fn new(header: &'a BlockHeader) -> Self {
        HeaderProofOfWork { header }
    }

    fn check(&self, p: &Params) -> Result {
        if is_valid_proof_of_work(p.max_bits(), self.header.bits, &self.header.hash()) {
            Ok(())
        } else {
            Err("Invalid proof-of-work (Block hash does not satisfy nBits)")
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
    header: &'a BlockHeader,
    current_time: u32,
}

impl<'a> HeaderTimestamp<'a> {
    fn new(header: &'a BlockHeader, current_time: u32) -> Self {
        HeaderTimestamp {
            header,
            current_time,
        }
    }

    fn check(&self, p: &Params) -> Result {
        if self.header.time > self.current_time + p.block_max_future() {
            error!("[HeaderTimestamp check]|Futuristic timestamp|header time{:}|current time:{:}|max_future{:?}", self.header.time, self.current_time, p.block_max_future());
            Err("Futuristic timestamp")
        } else {
            Ok(())
        }
    }
}
