//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]
// for encode/decode
// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;

// Needed for deriving `Encode` and `Decode` for `RawEvent`.
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
// Needed for the set of mock primitives used in our tests.
#[cfg(feature = "std")]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;
// Needed for tests (`with_externalities`).
#[cfg(test)]
#[cfg(feature = "std")]
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_system as system;
extern crate srml_balances as balances;
extern crate srml_timestamp as timestamp;

// bitcoin-rust
extern crate primitives;
extern crate chain;
extern crate serialization as ser;
extern crate bitcrypto;
extern crate merkle;
extern crate bit_vec;

#[cfg(test)]
mod tests;

mod verify_header;
mod blockchain;
mod tx;

use codec::Decode;
use rstd::prelude::*;
//use rstd::result::Result as StdResult;
use runtime_support::dispatch::Result;
use runtime_support::{StorageValue, StorageMap};
use runtime_primitives::traits::OnFinalise;

use system::ensure_signed;

use ser::deserialize;
use chain::{BlockHeader, Transaction};
use primitives::{hash::H256, compact::Compact};

pub use blockchain::BestHeader;

use blockchain::{Chain};
use tx::{UTXOIndex, validate_transaction, handle_input, handle_output};
pub use tx::RelayTx;


pub trait Trait: system::Trait + balances::Trait + timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as balances::Trait>::Balance
    {
        Fee(AccountId, Balance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn push_header(origin, header: Vec<u8>) -> Result;
        fn push_transaction(origin, tx: Vec<u8>) -> Result;
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        // do nothing
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Params {
    max_bits: u32,
    //Compact
    block_max_future: u32,
    max_fork_route_preset: u32,

    target_timespan_seconds: u32,
    target_spacing_seconds: u32,
    retargeting_factor: u32,

    double_spacing_seconds: u32,

    retargeting_interval: u32,
    min_timespan: u32,
    max_timespan: u32,
}

impl Params {
    pub fn new(max_bits: u32, block_max_future: u32, max_fork_route_preset: u32,
               target_timespan_seconds: u32, target_spacing_seconds: u32, retargeting_factor: u32) -> Params {
        Params {
            max_bits: max_bits,
            block_max_future: block_max_future,
            max_fork_route_preset: max_fork_route_preset,

            target_timespan_seconds: target_timespan_seconds,
            target_spacing_seconds: target_spacing_seconds,
            retargeting_factor: retargeting_factor,

            double_spacing_seconds: target_spacing_seconds / 10,

            retargeting_interval: target_timespan_seconds / target_spacing_seconds,
            min_timespan: target_timespan_seconds / retargeting_factor,
            max_timespan: target_timespan_seconds * retargeting_factor,
        }
    }

    pub fn max_bits(&self) -> Compact {
        Compact::new(self.max_bits)
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as BridgeOfBTC {
        // =====
        // header
        pub BestIndex get(best_index): BestHeader;

        // all valid blockheader (include orphan blockheader)
        pub BlockHeaderFor get(block_header_for): map H256 => Option<(BlockHeader, T::AccountId)>;

        // only main chain could has this number
        pub HeaderNumberFor get(header_num_for): map H256 => Option<u32>;
        pub HashForNumber get(hash_for_number): map u32 => H256;
//        pub OrphanIndexFor get(orphan_index_for): map H256 => Vec<H256>;

        // basic
        pub GenesisInfo get(genesis_info) config(genesis): (BlockHeader, u32);
        pub ParamsInfo get(params_info) config(): Params;

        // =====
        // tx
        pub ReceivePubkey get(receive_pubkey) config(): Option<Vec<u8>>;
        pub ReceivePubkeyHash get(receive_pubkeyhash) config(): Option<Vec<u8>>;

        pub UTXOSet get(utxo_set): map UTXOIndex => Option<u64>;
        pub TxSet get(tx_set): map H256 => Option<(Transaction, T::AccountId)>;
        pub BlockTxids get(block_txids): map H256 => Vec<H256>;

        // =====
        // others
        pub Fee get(fee) config(): T::Balance;
    }
    add_extra_genesis {
        build(|storage: &mut runtime_primitives::StorageMap, config: &GenesisConfig<T>| {
            use codec::Encode;
            let (genesis, number): (BlockHeader, u32) = config.genesis.clone();
            let h = genesis.hash();
            let who: T::AccountId = Default::default();
            // insert genesis
            storage.insert(GenesisConfig::<T>::hash(&<BlockHeaderFor<T>>::key_for(&h)).to_vec(),
                (genesis, who).encode());
            storage.insert(GenesisConfig::<T>::hash(&<HeaderNumberFor<T>>::key_for(&h)).to_vec(),
                number.encode());
            storage.insert(GenesisConfig::<T>::hash(&<HashForNumber<T>>::key_for(number)).to_vec(),
                h.encode());

            let best = BestHeader { number: number, hash: h };
            storage.insert(GenesisConfig::<T>::hash(&<BestIndex<T>>::key()).to_vec(), best.encode());
        });
    }
}

impl<T: Trait> Module<T> {
    // event
    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
}

impl<T: Trait> Module<T> {
    // public call
    pub fn push_header(origin: T::Origin, header: Vec<u8>) -> Result {
        let from = ensure_signed(origin)?;
        // parse header
        let header: BlockHeader = deserialize(header.as_slice()).map_err(|_| "can't deserialize the header vec")?;
        Self::process_header(header, &from)?;
        Ok(())
    }

    pub fn push_transaction(origin: T::Origin, tx: Vec<u8>) -> Result {
        let from = ensure_signed(origin)?;

        let tx: RelayTx = Decode::decode(&mut tx.as_slice()).ok_or("parse RelayTx err")?;
        Self::process_tx(tx, &from)?;
        Ok(())
    }
}


impl<T: Trait> Module<T> {
    pub fn process_header(header: BlockHeader, who: &T::AccountId) -> Result {
        // Check for duplicate
        if let Some(_) = Self::block_header_for(&header.hash()) {
            return Err("already store this header");
        }

        // orphan block check
        if <BlockHeaderFor<T>>::exists(&header.previous_header_hash) == false {
            return Err("can't find the prev header in ChainX, may be a orphan block");
        }
        // check
        {
            let c = verify_header::HeaderVerifier::new::<T>(&header).map_err(|e| e.info())?;
            c.check::<T>()?;
        }
        // insert valid header into storage
        <BlockHeaderFor<T>>::insert(header.hash(), (header.clone(), who.clone()));

        <Chain<T>>::insert_best_header(header).map_err(|e| e.info())?;

        Ok(())
    }

    pub fn process_tx(tx: RelayTx, who: &T::AccountId) -> Result {
        let receive_pubkeyhash: Vec<u8> = if let Some(h) = <ReceivePubkeyHash<T>>::get() { h } else {
            return Err("should set RECEIVE_PUBKEYHASH first");
        };

        let receive_pubkey: Vec<u8> = if let Some(h) = <ReceivePubkey<T>>::get() { h } else {
            return Err("should set RECEIVE_PUBKEY first");
        };

        validate_transaction::<T>(&tx, who, &receive_pubkeyhash)?;
//        let _ = handle_input(&tx.raw, &receive_pubkey);
        let _ = handle_output::<T>(&tx.raw, &receive_pubkeyhash);

        Ok(())
    }
}