// Copyright 2018 Chainpool.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]
// for encode/decode
// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
extern crate serde_derive;

// Needed for deriving `Encode` and `Decode` for `RawEvent`.
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

#[cfg(feature = "std")]
extern crate base58;
#[cfg(feature = "std")]
extern crate rustc_hex as hex;
#[cfg(feature = "std")]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;

extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.

#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

extern crate xrml_xassets_assets as xassets;
extern crate xrml_xassets_records as xrecords;
extern crate xrml_xsupport as xsupport;
#[cfg(test)]
extern crate xrml_xsystem as xsystem;

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
mod utils;
mod verify_header;

use codec::Decode;
use rstd::prelude::*;
use rstd::result::Result as StdResult;

use primitives::compact::Compact;
use primitives::hash::H256;

use chain::{BlockHeader, Transaction as BTCTransaction};

//use runtime_primitives::traits::OnFinalise;
use runtime_support::dispatch::{Parameter, Result};
use runtime_support::{StorageMap, StorageValue};
use ser::deserialize;
use system::ensure_signed;

use xassets::{Chain as ChainDef, ChainT};
use xsupport::storage::linked_node::{LinkedNodeCollection, Node, NodeIndex, NodeT};

pub use blockchain::BestHeader;
use blockchain::Chain;
use keys::DisplayLayout;
pub use keys::{Address, Error as AddressError};
pub use tx::RelayTx;
use tx::{handle_cert, handle_input, handle_output, handle_proposal, validate_transaction, UTXO};

pub trait Trait: system::Trait + balances::Trait + timestamp::Trait + xrecords::Trait {
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
        pub fn push_header(origin, header: Vec<u8>) -> Result {
            runtime_io::print("[bridge_btc] push btc header");
            let from = ensure_signed(origin)?;
            // parse header
            let header: BlockHeader =
                deserialize(header.as_slice()).map_err(|_| "can't deserialize the header vec")?;
            Self::process_header(header, &from)?;
            Ok(())
        }

        pub fn push_transaction(origin, tx: Vec<u8>) -> Result {
            runtime_io::print("[bridge_btc] push btc tx");
            let from = ensure_signed(origin)?;
            let tx: RelayTx = Decode::decode(&mut tx.as_slice()).ok_or("parse RelayTx err")?;
            Self::process_tx(tx, &from)?;
            Ok(())
        }

        pub fn propose_transaction(origin, tx: Vec<u8>) -> Result {
            runtime_io::print("[bridge_btc] propose btc tx");
            let from = ensure_signed(origin)?;

            let tx: BTCTransaction =
                Decode::decode(&mut tx.as_slice()).ok_or("parse transaction err")?;
            Self::process_btc_tx(tx, &from)?;
            Ok(())
        }
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"BTC";

    fn chain() -> ChainDef {
        ChainDef::BTC
    }

    fn check_addr(addr: &[u8], _: &[u8]) -> Result {
        Self::verify_btc_address(addr).map_err(|_| "verify btc addr err")?;
        Ok(())
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
    SendCert,
}

impl Default for TxType {
    fn default() -> Self {
        TxType::Deposit
    }
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct CandidateTx<AccountId: Parameter + Ord + Default> {
    pub tx: BTCTransaction,
    pub unexpect: bool,
    pub confirmed: bool,
    pub block_hash: H256,
    pub outs: Vec<(AccountId, u32)>,
    pub proposers: Vec<AccountId>,
}

impl<AccountId: Parameter + Ord + Default> CandidateTx<AccountId> {
    pub fn new(tx: BTCTransaction, outs: Vec<(AccountId, u32)>) -> Self {
        CandidateTx {
            tx,
            unexpect: false,
            confirmed: false,
            block_hash: Default::default(),
            outs,
            proposers: Vec::new(),
        }
    }
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct BTCTxLog {
    pub tx_type: TxType,
    pub tx: BTCTransaction,
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct BindInfo<AccountId: Parameter + Ord + Default> {
    pub account: AccountId,
    pub channel: Vec<u8>,
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct DepositInfo<AccountId: Parameter + Ord + Default> {
    pub account: AccountId,
    pub btc_balance: u64,
    pub tx_hash: H256,
    pub block_hash: H256,
    pub channel: Vec<u8>,
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct DepositHistInfo {
    pub btc_balance: u64,
    pub tx_hash: H256,
    pub block_hash: H256,
    pub channel: Vec<u8>,
}

impl NodeT for BTCTxLog {
    type Index = H256;
    fn index(&self) -> H256 {
        self.tx.hash()
    }
}

struct LinkedNodes<T: Trait>(runtime_support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedNodes<T> {
    type Header = TxSetHeader<T>;
    type NodeMap = TxSet<T>;
    type Tail = TxSetTail<T>;
}

decl_storage! {
    trait Store for Module<T: Trait> as BridgeOfBTC {
        /// get bestheader
        pub BestIndex get(best_index): BestHeader;

        /// all valid blockheader (include orphan blockheader)
        pub BlockHeaderFor get(block_header_for): map H256 => Option<(BlockHeader, T::AccountId, T::BlockNumber)>;

        /// only main chain could has this number
        /// get number by blockhash
        pub NumberForHash get(num_for_hash): map H256 => Option<u32>;
        /// get blockhash by number
        pub HashsForNumber get(hashs_for_num): map u32 => Vec<H256>;

        /// get GenesisInfo from genesis_config
        pub GenesisInfo get(genesis_info) config(genesis): (BlockHeader, u32);

        /// get ParamsInfo from genesis_config
        pub ParamsInfo get(params_info) config(): Params;

        ///  get NetworkId from genesis_config
        pub NetworkId get(network_id) config(): u32;

        /// get TrusteeAddress from genesis_config
        pub TrusteeAddress get(trustee_address) config(): Option<keys::Address>;

        /// get TrusteeRedeemScript from genesis_config
        pub TrusteeRedeemScript get(trustee_redeem_script) config(): Option<Vec<u8>>;

        /// get CertAddress from genesis_config
        pub CertAddress get(cert_address) config(): Option<keys::Address>;

        /// get CertRedeemScript from genesis_config
        pub CertRedeemScript get(cert_redeem_script) config(): Option<Vec<u8>>;

        /// utxo list
        pub UTXOSet get(utxos): map u64 => UTXO;
        pub UTXOSetLen get(utxo_len) config(): u64;

        /// get IrrBlock from genesis_config
        pub IrrBlock get(irr_block) config(): u32;

        /// get BtcFee from genesis_config
        pub BtcFee get(btc_fee) config(): u64;

        /// btc all related transactions set, use TxSetTail or TxSetHeader could iter them
        TxSetHeader get(tx_list_header): Option<NodeIndex<BTCTxLog>>;
        TxSetTail get(tx_list_tail): Option<NodeIndex<BTCTxLog>>;
        TxSet get(tx_set): map H256 => Option<Node<BTCTxLog>>;

        pub BlockTxsMapKeys get(block_txids): map H256 => Vec<H256>;

        /// get accountid by btc-address
        pub AddressMap get(address_map): map Address => Option<BindInfo<T::AccountId>>;

        /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
        pub TxProposalLen get(tx_proposal_len): u32;
        pub TxProposal get(tx_proposal): map u32 => Option<CandidateTx<T::AccountId>>;

        /// account, btc value, txhash, blockhash
        pub DepositCache get(deposit_cache): Option<Vec<DepositInfo<T::AccountId>>>;

        /// tx_hash, utxo index, btc value, blockhash
        pub DepositRecordsMap get(deposit_records): map Address => Option<Vec<DepositHistInfo>>;

        /// get cert info (cert_name, frozen_duration, cert_owner)
        pub CertCache get(cert_cache): Option<Vec<(Vec<u8>, u32, T::AccountId)>>;
        pub Fee get(fee) config(): T::Balance;
    }
    add_extra_genesis {
        build(|storage: &mut runtime_primitives::StorageMap, _: &mut runtime_primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
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
    #[allow(unused)]
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
}

impl<T: Trait> Module<T> {
    pub fn verify_btc_address(data: &[u8]) -> StdResult<Address, AddressError> {
        let r = b58::from(data.to_vec()).map_err(|_| AddressError::InvalidAddress)?;
        Address::from_layout(&r)
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
            runtime_io::print("check header");
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
        let trustee_address: Address = if let Some(h) = <TrusteeAddress<T>>::get() {
            h
        } else {
            return Err("should set RECEIVE_address first");
        };
        let cert_address: keys::Address = if let Some(h) = <CertAddress<T>>::get() {
            h
        } else {
            return Err("should set CERT_address first");
        };
        let tx_type = validate_transaction::<T>(&tx, (&trustee_address, &cert_address))?;
        match tx_type {
            TxType::Withdraw => {
                handle_input::<T>(&tx.raw, &tx.block_hash, &who, &trustee_address);
            }
            TxType::SendCert => {
                handle_cert::<T>(&tx.raw);
            }
            _ => {
                handle_output::<T>(&tx.raw, &tx.block_hash, &tx.previous_raw, &trustee_address);
            }
        }

        Ok(())
    }

    pub fn process_btc_tx(tx: BTCTransaction, who: &T::AccountId) -> Result {
        handle_proposal::<T>(tx, who)
    }
}
