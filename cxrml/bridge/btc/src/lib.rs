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
extern crate base58;
#[cfg(feature = "std")]
extern crate rustc_hex as hex;
#[cfg(feature = "std")]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;
// Needed for tests (`with_externalities`).
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
#[cfg(test)]
extern crate cxrml_associations as associations;
extern crate cxrml_funds_financialrecords as financial_records;
#[cfg(test)]
extern crate cxrml_support as cxsupport;
#[cfg(test)]
extern crate cxrml_system as cxsystem;
extern crate cxrml_tokenbalances as tokenbalances;
extern crate srml_balances as balances;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

// bitcoin-rust
extern crate bit_vec;
extern crate bitcrypto;
extern crate chain;
extern crate keys;
extern crate merkle;
extern crate primitives;
extern crate script;
extern crate serialization as ser;

#[cfg(test)]
mod tests;

mod b58;
mod blockchain;
mod tx;
mod verify_header;

use chain::{BlockHeader, Transaction as BTCTransaction};
use codec::Decode;
use primitives::compact::Compact;
use primitives::hash::H256;
use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_primitives::traits::OnFinalise;
use runtime_support::dispatch::{Parameter, Result};
use runtime_support::{StorageMap, StorageValue};
use ser::deserialize;
use system::ensure_signed;

pub use blockchain::BestHeader;
use blockchain::Chain;
use keys::DisplayLayout;
pub use keys::{Address, Error as AddressError};
pub use tx::RelayTx;
use tx::{handle_input, handle_output, handle_proposal, validate_transaction, UTXO};

pub trait Trait:
    system::Trait + balances::Trait + timestamp::Trait + financial_records::Trait
{
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
        fn propose_transaction(origin, tx: Vec<u8>) -> Result;
    }
}

impl<T: Trait> tokenbalances::TokenT for Module<T> {
    const SYMBOL: &'static [u8] = b"btc";
    fn check_addr(addr: &[u8], _: &[u8]) -> Result {
        Self::verify_btc_address(addr).map_err(|_| "verify btc addr err")?;
        Ok(())
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
    pub fn new(
        max_bits: u32,
        block_max_future: u32,
        max_fork_route_preset: u32,
        target_timespan_seconds: u32,
        target_spacing_seconds: u32,
        retargeting_factor: u32,
    ) -> Params {
        Params {
            max_bits,
            block_max_future,
            max_fork_route_preset,

            target_timespan_seconds,
            target_spacing_seconds,
            retargeting_factor,

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

#[derive(PartialEq, Clone, Copy, Encode, Decode)]
pub enum TxType {
    Withdraw,
    Deposit,
    Register,
    RegisterDeposit,
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct CandidateTx<AccountId: Parameter + Ord + Default> {
    pub proposer: Vec<AccountId>,
    pub tx: BTCTransaction,
    pub perfection: bool,
    pub block_hash: H256,
}

impl Default for TxType {
    fn default() -> Self {
        TxType::Deposit
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as BridgeOfBTC {
        // =====
        // header
        pub BestIndex get(best_index): BestHeader;

        // all valid blockheader (include orphan blockheader)
        pub BlockHeaderFor get(block_header_for): map H256 => Option<(BlockHeader, T::AccountId, T::BlockNumber)>;

        // only main chain could has this number
        pub NumberForHash get(num_for_hash): map H256 => Option<u32>;
        pub HashsForNumber get(hashs_for_num): map u32 => Vec<H256>;

        // basic
        pub GenesisInfo get(genesis_info) config(genesis): (BlockHeader, u32);
        pub ParamsInfo get(params_info) config(): Params;
        pub NetworkId get(network_id) config(): u32;

        // =====
        // tx
        pub ReceiveAddress get(receive_address) config(): Option<Address>;
        pub RedeemScript get(redeem_script) config(): Option<Vec<u8>>;

        pub UTXOSet get(utxo_set): map u64 => UTXO;
        pub UTXOMaxIndex get(utxo_max_index) config(): u64;
        pub IrrBlock get(irr_block) config(): u32;
        pub BtcFee get(btc_fee) config(): u64;
        pub TxSet get(tx_set): map H256 => Option<(T::AccountId, Address, TxType, u64, BTCTransaction)>; // Address, type, balance
        pub BlockTxids get(block_txids): map H256 => Vec<H256>;
        pub AddressMap get(address_map): map Address => Option<T::AccountId>;
        pub AccountMap get(account_map): map T::AccountId => Option<Address>;
        pub TxProposal get(tx_proposal): Option<CandidateTx<T::AccountId>>;
        pub DepositCache get(deposit_cache): Option<Vec<(T::AccountId, u64, H256)>>; // account_id, amount, H256

        pub AccountsMaxIndex get(accounts_max_index) config(): u64;
        pub AccountsSet get(accounts_set): map u64 => Option<(H256, Address, T::AccountId, T::BlockNumber, TxType)>;

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
            let block_number: T::BlockNumber = Default::default();

            // check blocknumber is a new epoch
            if config.network_id == 0 {
                if number % config.params_info.retargeting_interval != 0 {
                    panic!("the blocknumber[{:}] should start from a changed difficulty block", number);
                }
            }

            // insert genesis
            storage.insert(GenesisConfig::<T>::hash(&<BlockHeaderFor<T>>::key_for(&h)).to_vec(),
                (genesis, who, block_number).encode());
            storage.insert(GenesisConfig::<T>::hash(&<NumberForHash<T>>::key_for(&h)).to_vec(),
                number.encode());
            storage.insert(GenesisConfig::<T>::hash(&<HashsForNumber<T>>::key_for(number)).to_vec(),
                [h.clone()].to_vec().encode());

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
        let header: BlockHeader =
            deserialize(header.as_slice()).map_err(|_| "can't deserialize the header vec")?;
        Self::process_header(header, &from)?;
        Ok(())
    }

    pub fn push_transaction(origin: T::Origin, tx: Vec<u8>) -> Result {
        let from = ensure_signed(origin)?;

        let tx: RelayTx = Decode::decode(&mut tx.as_slice()).ok_or("parse RelayTx err")?;
        Self::process_tx(tx, &from)?;
        Ok(())
    }

    pub fn propose_transaction(origin: T::Origin, tx: Vec<u8>) -> Result {
        let from = ensure_signed(origin)?;

        let tx: BTCTransaction =
            Decode::decode(&mut tx.as_slice()).ok_or("parse transaction err")?;
        Self::process_btc_tx(tx, &from)?;
        Ok(())
    }
}

impl<T: Trait> Module<T> {
    pub fn verify_btc_address(data: &[u8]) -> StdResult<Address, AddressError> {
        Address::from_layout(data)
    }

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
        <BlockHeaderFor<T>>::insert(
            header.hash(),
            (
                header.clone(),
                who.clone(),
                <system::Module<T>>::block_number(),
            ),
        );

        <Chain<T>>::insert_best_header(header).map_err(|e| e.info())?;

        Ok(())
    }

    pub fn process_tx(tx: RelayTx, who: &T::AccountId) -> Result {
        let receive_address: Address = if let Some(h) = <ReceiveAddress<T>>::get() {
            h
        } else {
            return Err("should set RECEIVE_address first");
        };

        let tx_type = validate_transaction::<T>(&tx, &receive_address).unwrap();
        match tx_type {
            TxType::Withdraw => {
                handle_input::<T>(&tx.raw, &tx.block_hash, &who, &receive_address);
            }
            _ => {
                let _utxos = handle_output::<T>(
                    &tx.raw,
                    &tx.block_hash,
                    &who,
                    &tx.previous_raw,
                    &receive_address,
                );
            }
        }

        Ok(())
    }

    pub fn process_btc_tx(tx: BTCTransaction, who: &T::AccountId) -> Result {
        handle_proposal::<T>(tx, who)
    }
}
