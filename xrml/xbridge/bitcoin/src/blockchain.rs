// Copyright 2019 Chainpool.

use primitives::hash::H256;
use rstd::marker::PhantomData;
use rstd::result::Result;
use runtime_support::StorageMap;
use tx::handle_tx;
use {BlockHeaderFor, BlockHeaderInfo, Trait};

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
            None => return Err(ChainErr::OtherErr("Not found block header for this hash")),
        };

        info!("Confirmed header: {:}  {:}", confirmed_header.height as u64, hash);
        let tx_list = confirmed_header.txid;
        for txid in tx_list {
            // deposit & withdraw
            match handle_tx::<T>(&txid) {
                Err(_) => {
                    info!("Handle tx failed: {:}", txid);
                }
                Ok(()) => (),
            }
        }
        Ok(())
    }
}
