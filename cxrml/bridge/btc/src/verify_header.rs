// Copyright 2018 Chainpool.

use rstd::cmp;
use rstd::result::Result as StdResult;

use chain::BlockHeader;
use primitives::compact::Compact;
use primitives::hash::H256;
use primitives::U256;

use super::{
    BlockHeaderFor, GenesisInfo, HashsForNumber, NetworkId, NumberForHash, Params, ParamsInfo,
    Trait,
};
use blockchain::ChainErr;
use runtime_primitives::traits::As;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};
<<<<<<< HEAD
use runtime_primitives::traits::As;
use super::{Trait, NumberForHash, HashsForNumber, BlockHeaderFor, ParamsInfo, NetworkId, Params, GenesisInfo};
use blockchain::ChainErr;
=======
use timestamp;
>>>>>>> develop

pub struct HeaderVerifier<'a> {
    pub work: HeaderWork<'a>,
    pub proof_of_work: HeaderProofOfWork<'a>,
    pub timestamp: HeaderTimestamp<'a>,
}

impl<'a> HeaderVerifier<'a> {
    pub fn new<T: Trait>(header: &'a BlockHeader) -> StdResult<Self, ChainErr> {
        let params: Params = <ParamsInfo<T>>::get();

        let mut prev_height = 0;
        let mut prev_hash = header.previous_header_hash.clone();
        let mut count = 0;
        for i in 0..params.max_fork_route_preset {
            if let Some(h) = <NumberForHash<T>>::get(&prev_hash) {
                count = i;
                prev_height = h + i;
                break;
            } else {
                prev_hash = <BlockHeaderFor<T>>::get(&prev_hash)
                    .unwrap()
                    .0
                    .previous_header_hash
                    .clone();
            }
        }
        if count == params.max_fork_route_preset {
            return Err(ChainErr::AncientFork);
        }

        let this_height = prev_height + 1;
        let now: T::Moment = <timestamp::Now<T>>::get();
        let current_time: u32 = now.as_() as u32;

        Ok(HeaderVerifier {
            work: HeaderWork::new(header, this_height),
            proof_of_work: HeaderProofOfWork::new(header),
            timestamp: HeaderTimestamp::new(header, current_time, params.block_max_future),
        })
    }

    pub fn check<T: Trait>(&self) -> Result {
        let params: Params = ParamsInfo::<T>::get();
        let network_id: u32 = NetworkId::<T>::get();
        if network_id == 0 {
            self.work.check::<T>(&params)?;
        }
        self.proof_of_work.check(&params)?;
        self.timestamp.check()?;
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
        if work == self.header.bits {
            Ok(())
        } else {
            Err("nBits do not match difficulty rules")
        }
    }
}

pub fn work_required<T: Trait>(parent_hash: H256, height: u32, params: &Params) -> Compact {
    let max_bits = params.max_bits().into();
    if height == 0 {
        return max_bits;
    }

    let parent_header: BlockHeader = <BlockHeaderFor<T>>::get(&parent_hash).unwrap().0;

    if is_retarget_height(height, params) {
        return work_required_retarget::<T>(parent_header, height, params);
    }

    parent_header.bits
}

pub fn is_retarget_height(height: u32, p: &Params) -> bool {
    height % p.retargeting_interval == 0
}

/// Algorithm used for retargeting work every 2 weeks
pub fn work_required_retarget<T: Trait>(
    parent_header: BlockHeader,
    height: u32,
    params: &Params,
) -> Compact {
    let retarget_num = height - params.retargeting_interval;

    let (genesis_header, genesis_num) = <GenesisInfo<T>>::get();
    let mut retarget_header = parent_header.clone();
    if retarget_num < genesis_num {
        retarget_header = genesis_header;
    } else {
        let hash_list = <HashsForNumber<T>>::get(&retarget_num);
        for h in hash_list {
            // look up in main chain
            if <NumberForHash<T>>::get(&h).is_some() {
                retarget_header = <BlockHeaderFor<T>>::get(&h).unwrap().0;
                break;
            }
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
    retarget = retarget / U256::from(params.target_timespan_seconds);
    //    retarget = retarget / U256::from(TARGET_TIMESPAN_SECONDS);
    if retarget > maximum {
        params.max_bits()
    } else {
        retarget.into()
    }
}

/// Returns constrained number of seconds since last retarget
pub fn retarget_timespan(retarget_timestamp: u32, last_timestamp: u32, p: &Params) -> u32 {
    // subtract unsigned 32 bit numbers in signed 64 bit space in
    // order to prevent underflow before applying the range constraint.
    let timespan = last_timestamp as i64 - retarget_timestamp as i64;
    range_constrain(timespan, p.min_timespan as i64, p.max_timespan as i64) as u32
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

pub fn is_valid_proof_of_work(max_work_bits: Compact, bits: Compact, hash: &H256) -> bool {
    let maximum = match max_work_bits.to_u256() {
        Ok(max) => max,
        _err => return false,
    };

    let target = match bits.to_u256() {
        Ok(target) => target,
        _err => return false,
    };

    let value = U256::from(&*hash.reversed() as &[u8]);
    target <= maximum && value <= target
}

pub struct HeaderTimestamp<'a> {
    header: &'a BlockHeader,
    current_time: u32,
    max_future: u32,
}

impl<'a> HeaderTimestamp<'a> {
    fn new(header: &'a BlockHeader, current_time: u32, max_future: u32) -> Self {
        HeaderTimestamp {
            header,
            current_time,
            max_future,
        }
    }

    fn check(&self) -> Result {
        if self.header.time > self.current_time + self.max_future {
            Err("Futuristic timestamp")
        } else {
            Ok(())
        }
    }
}
