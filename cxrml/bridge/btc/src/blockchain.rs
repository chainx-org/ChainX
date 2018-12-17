// Copyright 2018 Chainpool.

use codec::Encode;
use rstd::marker::PhantomData;
use rstd::prelude::*;
use rstd::result::Result;
use runtime_io;
use runtime_primitives::traits::As;
use runtime_support::{StorageMap, StorageValue};
use {BtcFee, IrrBlock};

use chain::BlockHeader;
use financial_records;
use financial_records::Symbol;
use primitives::hash::H256;
use staking;

use {
    BestIndex, BlockHeaderFor, CertCache, DepositCache, HashsForNumber, Module, NumberForHash,
    Params, ParamsInfo, Trait, TxProposal,
};

use tx::{Proposal, RollBack, TxStorage};

use tokenbalances::TokenT;

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BestHeader {
    /// Height/number of the best block (genesis block has zero height)
    pub number: u32,
    /// Hash of the best block
    pub hash: H256,
}

#[derive(Clone)]
pub struct SideChainOrigin {
    /// newest ancestor block number
    pub ancestor: u32,
    /// side chain block hashes. Ordered from oldest to newest
    pub canonized_route: Vec<H256>,
    /// canon chain block hahses. Ordered from oldest to newest
    pub decanonized_route: Vec<H256>,
    /// new block number
    pub block_number: u32,
}

#[derive(Clone)]
pub enum BlockOrigin {
    KnownBlock,
    CanonChain { block_number: u32 },
    SideChain(SideChainOrigin),
    SideChainBecomingCanonChain(SideChainOrigin),
}

pub enum ChainErr {
    /// Invalid block
    CannotCanonize,
    /// Uknown parent
    UnknownParent,
    /// Not Found
    NotFound,
    /// Ancient fork
    AncientFork,
    /// unreachable,
    Unreachable,
    /// must zero
    CanonizeMustZero,
    DecanonizeMustZero,
    ForkErr,

    OtherErr(&'static str),
}

impl ChainErr {
    pub fn info(&self) -> &'static str {
        match *self {
            ChainErr::CannotCanonize => "Cannot canonize block",
            ChainErr::UnknownParent => "Block parent is unknown",
            ChainErr::NotFound => "Not to find orphaned side chain in header collection; qed",
            ChainErr::AncientFork => "Fork is too long to proceed",
            ChainErr::Unreachable => "Should not occur",
            ChainErr::CanonizeMustZero => "[canonize] must be zero in this case",
            ChainErr::DecanonizeMustZero => "[decanonize] must be zero in this case",
            ChainErr::ForkErr => "The hash should same",
            ChainErr::OtherErr(s) => s,
        }
    }
}

pub struct Chain<T: Trait>(PhantomData<T>);

impl<T: Trait> Chain<T> {
    pub fn insert_best_header(header: BlockHeader) -> Result<(), ChainErr> {
        let block_origin = Self::block_origin(&header)?;
        match block_origin {
            BlockOrigin::KnownBlock => {
                return Err(ChainErr::Unreachable);
            }
            // case 1: block has been added to the main branch
            BlockOrigin::CanonChain { .. } => {
                Self::canonize(&header.hash())?;
                runtime_io::print("[Main] block has been added to the main branch");
                Ok(())
            }
            // case 2: block has been added to the side branch with reorganization to this branch
            BlockOrigin::SideChainBecomingCanonChain(origin) => {
                Self::fork(origin.clone())?;
                Self::canonize(&header.hash())?;
                runtime_io::print("[Switch to Main] block has been added to the side branch with reorganization to this branch");
                Ok(())
            }
            // case 3: block has been added to the side branch without reorganization to this branch
            BlockOrigin::SideChain(_origin) => {
                runtime_io::print("[Side] block has been added to the side branch without reorganization to this branch");
                Ok(())
            }
        }
    }

    fn decanonize() -> Result<H256, ChainErr> {
        let best_index = <BestIndex<T>>::get();
        let best_hash = best_index.hash;
        let best_bumber = best_index.number;

        let best_header: BlockHeader =
            if let Some((header, _, _)) = <BlockHeaderFor<T>>::get(&best_hash) {
                header
            } else {
                return Err(ChainErr::OtherErr("not found blockheader for this hash"));
            };

        let new_best_header = BestHeader {
            hash: best_header.previous_header_hash.clone(),
            number: if best_bumber > 0 {
                best_bumber - 1
            } else {
                if best_header.previous_header_hash != Default::default() {
                    return Err(ChainErr::DecanonizeMustZero);
                }
                0
            },
        };

        // remove related tx
        TxStorage::<T>::rollback_tx(&best_hash).map_err(|s| ChainErr::OtherErr(s))?;

        <NumberForHash<T>>::remove(&best_hash);
        // do not need to remove HashsForNumber

        <BestIndex<T>>::put(new_best_header);
        Ok(best_hash)
    }

    fn canonize(hash: &H256) -> Result<(), ChainErr> {
        // get current best index
        let best_index = <BestIndex<T>>::get();
        let best_hash = best_index.hash;
        let best_number = best_index.number;

        let header: BlockHeader = if let Some((header, _, _)) = <BlockHeaderFor<T>>::get(hash) {
            header
        } else {
            return Err(ChainErr::OtherErr("not found blockheader for this hash"));
        };

        if best_hash != header.previous_header_hash {
            return Err(ChainErr::CannotCanonize);
        }

        let new_best_header = BestHeader {
            hash: hash.clone(),
            number: if header.previous_header_hash == Default::default() {
                if best_number != 0 {
                    return Err(ChainErr::CanonizeMustZero);
                }
                0
            } else {
                best_number + 1
            },
        };

        <NumberForHash<T>>::insert(new_best_header.hash.clone(), new_best_header.number);
        <HashsForNumber<T>>::mutate(new_best_header.number, |v| {
            let h = new_best_header.hash.clone();
            if v.contains(&h) == false {
                v.push(h);
            }
        });
        runtime_io::print("best header");
        runtime_io::print(new_best_header.number as u64);
        // best chain choose finish
        <BestIndex<T>>::put(&new_best_header);

        // deposit/withdraw handle start
        let symbol: Symbol = Module::<T>::SYMBOL.to_vec();
        let irr_block = <IrrBlock<T>>::get();
        // Deposit
        if let Some(vec) = <DepositCache<T>>::take() {
            runtime_io::print("deposit start ---accountid---amount---tx_hash---blocknum");
            let mut uncomplete_cache: Vec<(T::AccountId, u64, H256, H256)> = Vec::new();
            for (account_id, amount, tx_hash, block_hash) in vec {
                match <NumberForHash<T>>::get(block_hash.clone()) {
                    Some(height) => {
                        if new_best_header.number >= height + irr_block {
                            runtime_io::print(account_id.encode().as_slice());
                            runtime_io::print(amount);
                            runtime_io::print(&tx_hash[..]);
                            runtime_io::print(height as u64);
                            // TODO handle err
                            let _ = <financial_records::Module<T>>::deposit(
                                &account_id,
                                &symbol,
                                As::sa(amount),
                                Some(tx_hash.as_ref().to_vec()),
                            );
                        } else {
                            runtime_io::print("not reach irr_block --best --height");
                            runtime_io::print(new_best_header.number as u64);
                            runtime_io::print(height as u64);
                            uncomplete_cache.push((account_id, amount, tx_hash, block_hash));
                        }
                    }
                    None => {
                        // TODO 遇到分叉，需要从deposit cache剔除相应的交易
                        uncomplete_cache.push((account_id, amount, tx_hash, block_hash));
                    } // Optmise
                }
            }
            <DepositCache<T>>::put(uncomplete_cache);
        }

        // Withdraw
        let len = Module::<T>::tx_proposal_len();
        // get last proposal
        if len > 0 {
            let mut candidate = Module::<T>::tx_proposal(len - 1).unwrap();
            // candidate: CandidateTx
            if candidate.confirmed == false {
                runtime_io::print("withdraw start ---accountid---tx_hash---blocknum");
                match <NumberForHash<T>>::get(&candidate.block_hash) {
                    Some(height) => {
                        if new_best_header.number >= height + irr_block {
                            let txid = candidate.tx.hash();
                            for (account_id, _) in candidate.outs.clone() {
                                runtime_io::print(account_id.encode().as_slice());
                                runtime_io::print(&txid[..]);
                                runtime_io::print(height as u64);

                                // TODO handle err
                                let _ = <financial_records::Module<T>>::withdrawal_finish(
                                    &account_id,
                                    &symbol,
                                    Some(txid.as_ref().to_vec()),
                                );
                            }
                            candidate.confirmed = true;
                            // mark this tx withdraw finish!
                            TxProposal::<T>::insert(len - 1, candidate);
                        } else {
                            runtime_io::print("not reach irr_block --best --height");
                            runtime_io::print(new_best_header.number as u64);
                            runtime_io::print(height as u64);
                        }
                    }
                    None => {
                        // todo 处理分叉问题
                    }
                }
            }
        }

        // case 0: 当刚启动时Candidate lenth = 0 时
        // case 1: 所有提现交易都是正常逻辑执行，会confirmed.
        // case 2:  非正常逻辑提现，candidate.unexpect 会在handle_input时设置，
        // 标记该链上这笔proposal由于BTC 托管人没有按着正常逻辑签名广播， 该proposal可能永远不会confirmed.
        // 所以开始重新创建proposal.
        if len == 0 {
            runtime_io::print("crate_proposal case 1");
            // no withdraw cache would return None
            if let Some(indexs) = financial_records::Module::<T>::withdrawal_cache_indexs(&symbol) {
                let btc_fee = <BtcFee<T>>::get();
                if let Err(e) = <Proposal<T>>::create_proposal(indexs, btc_fee) {
                    return Err(ChainErr::OtherErr(e));
                }
            }
        }
        if len > 0 {
            let candidate = Module::<T>::tx_proposal(len - 1).unwrap();
            if candidate.confirmed || candidate.unexpect {
                runtime_io::print("crate_proposal case 2,3  ---confirmed---unexpect");
                runtime_io::print(candidate.confirmed.encode().as_slice());
                runtime_io::print(candidate.unexpect.encode().as_slice());
                // no withdraw cache would return None
                if let Some(indexs) =
                    financial_records::Module::<T>::withdrawal_cache_indexs(&symbol)
                {
                    let btc_fee = <BtcFee<T>>::get();
                    if let Err(e) = <Proposal<T>>::create_proposal(indexs, btc_fee) {
                        return Err(ChainErr::OtherErr(e));
                    }
                }
            }
        }

        // SendCert
        if let Some(cert_info) = <CertCache<T>>::take() {
            runtime_io::print("send cert start");
            if let Err(e) = <staking::Module<T>>::issue(cert_info.0, cert_info.1, cert_info.2) {
                return Err(ChainErr::OtherErr(e));
            }
        }

        Ok(())
    }
    /// Rollbacks single best block
    #[allow(unused)]
    pub fn rollback_best() -> Result<H256, ChainErr> {
        Self::decanonize()
    }

    fn fork(side_chain: SideChainOrigin) -> Result<(), ChainErr> {
        for hash in side_chain.decanonized_route.into_iter().rev() {
            let decanonized_hash = Self::decanonize()?;

            if hash != decanonized_hash {
                return Err(ChainErr::ForkErr);
            }
        }

        for block_hash in &side_chain.canonized_route {
            Self::canonize(block_hash)?;
        }

        Ok(())
    }

    fn block_origin(header: &BlockHeader) -> Result<BlockOrigin, ChainErr> {
        let best_index: BestHeader = <BestIndex<T>>::get();

        let best_header: BlockHeader =
            if let Some((header, _, _)) = <BlockHeaderFor<T>>::get(&best_index.hash) {
                header
            } else {
                return Err(ChainErr::OtherErr("not found blockheader for this hash"));
            };

        if <NumberForHash<T>>::exists(header.hash()) {
            return Ok(BlockOrigin::KnownBlock);
        }

        if best_header.hash() == header.previous_header_hash {
            return Ok(BlockOrigin::CanonChain {
                block_number: best_index.number + 1,
            });
        }

        if <BlockHeaderFor<T>>::exists(&header.previous_header_hash) == false {
            return Err(ChainErr::UnknownParent);
        }

        let params: Params = <ParamsInfo<T>>::get();

        let mut sidechain_route = Vec::new();
        let mut next_hash = header.previous_header_hash.clone();
        for fork_len in 0..params.max_fork_route_preset {
            let num = <NumberForHash<T>>::get(&next_hash);
            match num {
                None => {
                    sidechain_route.push(next_hash.clone());
                    if let Some(header) = <BlockHeaderFor<T>>::get(&next_hash) {
                        next_hash = header.0.previous_header_hash;
                    } else {
                        return Err(ChainErr::NotFound);
                    }
                }
                Some(number) => {
                    // find common ancestor in main chain
                    let block_number = number + fork_len + 1;
                    let origin = SideChainOrigin {
                        ancestor: number,
                        canonized_route: sidechain_route.into_iter().rev().collect(),
                        decanonized_route: (number + 1..best_index.number + 1)
                            .into_iter()
                            .filter_map(|decanonized_bn| {
                                let hash_list = <HashsForNumber<T>>::get(decanonized_bn);
                                for h in hash_list {
                                    // look up in main chain
                                    if <NumberForHash<T>>::get(&h).is_some() {
                                        return Some(h);
                                    }
                                }
                                None
                            })
                            .collect(),
                        block_number,
                    };
                    if block_number > best_index.number {
                        return Ok(BlockOrigin::SideChainBecomingCanonChain(origin));
                    } else {
                        return Ok(BlockOrigin::SideChain(origin));
                    }
                }
            }
        }

        Err(ChainErr::AncientFork)
    }
}
