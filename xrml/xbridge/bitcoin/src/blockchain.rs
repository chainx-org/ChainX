// Copyright 2019 Chainpool.
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
    pub fn handle_confirm_block(confirmed_header: BlockHeaderInfo) -> Result<(), ChainErr> {
        let hash = confirmed_header.header.hash();
        let confirmed_header: BlockHeaderInfo = match <BlockHeaderFor<T>>::get(&hash) {
            Some(header) => header,
            None => return Err(ChainErr::OtherErr("Not found block header for this hash")),
        };

        info!(
            "Confirmed: {:}  {:}...",
            confirmed_header.height as u64,
            &format!("0x{:?}", hash)[0..8]
        );
        let tx_list = confirmed_header.txid;
        for txid in tx_list {
            // deposit & withdraw
            match handle_tx::<T>(&txid) {
                Err(_) => {
                    info!("Handle tx failed: {:}...", &format!("0x{:?}", txid)[0..8]);
                }
                Ok(()) => (),
            }
        }
        Ok(())
    }
}
