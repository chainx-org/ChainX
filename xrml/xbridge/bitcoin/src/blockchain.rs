// Copyright 2018 Chainpool.

use primitives::hash::H256;
use rstd::marker::PhantomData;
use rstd::result::Result;
use runtime_io;
use runtime_support::{StorageMap, StorageValue};
use tx::handle_tx;
use tx::Proposal;
use xrecords;
use BtcFee;
use {BlockHeaderFor, BlockHeaderInfo, MaxWithdrawAmount, Module, Trait};

pub enum ChainErr {
    /// Uknown parent
    UnknownParent,
    /// Not Found
    NotFound,
    /// Ancient fork
    AncientFork,
    OtherErr(&'static str),
}

impl ChainErr {
    pub fn info(&self) -> &'static str {
        match *self {
            ChainErr::UnknownParent => "Block parent is unknown",
            ChainErr::NotFound => "Not to find orphaned side chain in header collection; qed",
            ChainErr::AncientFork => "Fork is too long to proceed",
            ChainErr::OtherErr(s) => s,
        }
    }
}

pub struct Chain<T: Trait>(PhantomData<T>);

impl<T: Trait> Chain<T> {
    pub fn update_header(confirmed_header: BlockHeaderInfo) -> Result<(), ChainErr> {
        Self::canonize(&confirmed_header.header.hash())?;
        Ok(())
    }

    fn canonize(hash: &H256) -> Result<(), ChainErr> {
        let confirmed_header: BlockHeaderInfo = match <BlockHeaderFor<T>>::get(hash) {
            Some(header) => header,
            None => return Err(ChainErr::OtherErr("not found blockheader for this hash")),
        };

        runtime_io::print("[bridge-btc] confirmed header height:");
        runtime_io::print(confirmed_header.height as u64);

        let tx_list = confirmed_header.txid;
        for txid in tx_list {
            runtime_io::print("[bridge-btc] handle confirmed_header's tx list");
            // deposit & bind & withdraw & cert
            match handle_tx::<T>(&txid) {
                Err(_) => {
                    runtime_io::print("[bridge-btc] handle_tx error, tx hash:");
                    runtime_io::print(&txid[..]);
                }
                Ok(()) => (),
            }
        }

        // Withdraw
        match Module::<T>::tx_proposal() {
            None => {
                let max_application_numbers = <MaxWithdrawAmount<T>>::get();
                // no withdraw cache would return None
                if let Some(indexs) = xrecords::Module::<T>::withdrawal_application_numbers(
                    xassets::Chain::Bitcoin,
                    max_application_numbers,
                ) {
                    let btc_fee = <BtcFee<T>>::get();
                    runtime_io::print("[bridge-btc] crate proposal...");
                    if let Err(e) = <Proposal<T>>::create_proposal(indexs, btc_fee) {
                        return Err(ChainErr::OtherErr(e));
                    }
                }
            }
            Some(_) => {
                runtime_io::print("[bridge-btc] still have Candidate not process");
            }
        }
        Ok(())
    }
}
