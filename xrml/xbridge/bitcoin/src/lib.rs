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

extern crate xrml_xaccounts as xaccounts;
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
mod header_proof;
mod tx;

pub use b58::from;
use blockchain::Chain;
use chain::{BlockHeader, Transaction as BTCTransaction};
use codec::Decode;
use keys::DisplayLayout;
use keys::{Address, Error as AddressError};
use primitives::compact::Compact;
use primitives::hash::H256;
use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};
use ser::deserialize;
use system::ensure_signed;
pub use tx::RelayTx;
use tx::{handle_tx, inspect_address, validate_transaction};
use xassets::{Chain as ChainDef, ChainT};

pub trait Trait:
    system::Trait + balances::Trait + timestamp::Trait + xrecords::Trait + xaccounts::Trait
{
    //    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

//decl_event!(
//    pub enum Event<T> where
//        <T as system::Trait>::AccountId,
//        <T as balances::Trait>::Balance
//    {
//
//    }
//);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        pub fn push_header(origin, header: Vec<u8>) -> Result {
            runtime_io::print("[bridge_btc] push btc header");

            let from = ensure_signed(origin)?;

            let header: BlockHeader = deserialize(header.as_slice()).map_err(|_| "Cannot deserialize the header vec")?;

            ensure!(
                Self::block_header_for(&header.hash()).is_none(),
                "Cannot push if the header already exists."
            );
            ensure!(
                <BlockHeaderFor<T>>::exists(&header.previous_header_hash),
                "Cannot push if can't find its previous header in ChainX, which may be header of some orphan block."
            );

            Self::apply_push_header(header, &from)?;

            Ok(())
        }

        pub fn push_transaction(origin, tx: Vec<u8>) -> Result {
            runtime_io::print("[bridge_btc] push btc tx");

            ensure_signed(origin)?;

            let tx: RelayTx = Decode::decode(&mut tx.as_slice()).ok_or("parse RelayTx err")?;
            let trustee_address = <TrusteeAddress<T>>::get().ok_or("Should set RECEIVE_address first.")?;
            let cert_address = <CertAddress<T>>::get().ok_or("Should set CERT_address first.")?;

            Self::apply_push_transaction(tx, trustee_address, cert_address)?;

            Ok(())
        }

//        pub fn propose_transaction(origin, tx: Vec<u8>) -> Result {
//            runtime_io::print("[bridge_btc] propose btc tx");
//            let from = ensure_signed(origin)?;
//
//            let tx: BTCTransaction =
//                Decode::decode(&mut tx.as_slice()).ok_or("parse transaction err")?;
//            Self::process_btc_tx(tx, &from)?;
//            Ok(())
//        }
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"BTC";

    fn chain() -> ChainDef {
        ChainDef::Bitcoin
    }

    fn check_addr(addr: &[u8], _: &[u8]) -> Result {
        Self::verify_btc_address(addr).map_err(|_| "verify btc addr err")?;
        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
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
pub struct CandidateTx {
    pub tx: BTCTransaction,
    pub outs: Vec<u32>,
}

impl CandidateTx {
    pub fn new(tx: BTCTransaction, outs: Vec<u32>) -> Self {
        CandidateTx { tx, outs }
    }
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct BlockHeaderInfo {
    pub header: BlockHeader,
    pub height: u32,
    pub confirmed: bool,
    pub txid: Vec<H256>,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
pub struct TxInfo {
    pub input_address: keys::Address,
    pub raw_tx: BTCTransaction,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
pub struct UTXO {
    pub txid: H256,
    pub index: u32,
    pub balance: u64,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
pub struct UTXOKey {
    pub txid: H256,
    pub index: u32,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
pub struct UTXOStatus {
    pub balance: u64,
    pub status: bool,
}

decl_storage! {
    trait Store for Module<T: Trait> as BridgeOfBTC {
        /// get bestheader
        pub BestIndex get(best_index): H256;

        /// all valid blockheader (include orphan blockheader)
        pub BlockHeaderFor get(block_header_for): map H256 => Option<BlockHeaderInfo>;
        pub BlockHeightFor get(block_height_for): map u32 => Option<Vec<H256>>;

        /// map for tx
        pub TxFor get(tx_for): map H256 => TxInfo;
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
        pub UTXOSet get(utxos): map UTXOKey => UTXOStatus;
        pub UTXOSetKey get(utxo_key): Option<Vec<UTXOKey>>;

        /// get IrrBlock from genesis_config
        pub ReservedBlock get(reserved) config(): u32;

        /// get IrrBlock from genesis_config
        pub IrrBlock get(irr_block) config(): u32;

        /// get BtcFee from genesis_config
        pub BtcFee get(btc_fee) config(): u64;

        pub MaxWithdrawAmount get(max_withdraw_amount) config(): u32;

        /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
        pub TxProposal get(tx_proposal): Option<CandidateTx>;

        /// tx_hash, utxo index, btc value, blockhash
        pub PendingDepositMap get(pending_deposit): map Address => Option<Vec<UTXOKey>>;

        /// get accountid by btc-address
        pub AddressMap get(address_map): map Address => Option<T::AccountId>;

        /// get btc-address list  by accountid
        pub AccountMap get(account_map): map T::AccountId => Option<Vec<Address>>;

    }
    add_extra_genesis {
        build(|storage: &mut runtime_primitives::StorageMap, _: &mut runtime_primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
            use codec::Encode;
            let (header, number): (BlockHeader, u32) = config.genesis.clone();
            let h = header.hash();

            if config.network_id == 0 {
                if number % config.params_info.retargeting_interval != 0 {
                    panic!("the blocknumber[{:}] should start from a changed difficulty block", number);
                }
            }
            let genesis = BlockHeaderInfo {
                header: header,
                height: number,
                confirmed: true,
                txid: [].to_vec(),
            };
            // insert genesis
            storage.insert(GenesisConfig::<T>::hash(&<BlockHeaderFor<T>>::key_for(&h)).to_vec(),
                genesis.encode());
            storage.insert(GenesisConfig::<T>::hash(&<BlockHeightFor<T>>::key_for(genesis.height)).to_vec(),
                [h.clone()].to_vec().encode());
            storage.insert(GenesisConfig::<T>::hash(&<BestIndex<T>>::key()).to_vec(), h.encode());
        });
    }
}

//impl<T: Trait> Module<T> {
// event
/// Deposit one of this module's events.
//    #[allow(unused)]
//    fn deposit_event(event: Event<T>) {
//        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
//    }
//}

impl<T: Trait> Module<T> {
    pub fn verify_btc_address(data: &[u8]) -> StdResult<Address, AddressError> {
        let r = b58::from(data.to_vec()).map_err(|_| AddressError::InvalidAddress)?;
        Address::from_layout(&r)
    }

    fn apply_push_header(header: BlockHeader, _who: &T::AccountId) -> Result {
        // check
        let c = header_proof::HeaderVerifier::new::<T>(&header).map_err(|e| e.info())?;
        c.check::<T>()?;

        let header_info = BlockHeaderInfo {
            header: header.clone(),
            height: c.get_height::<T>(),
            confirmed: false,
            txid: [].to_vec(),
        };

        // insert valid header into storage
        <BlockHeaderFor<T>>::insert(&header.hash(), header_info.clone());

        let mut height_hash: Vec<H256> =
            <BlockHeightFor<T>>::get(&header_info.height).unwrap_or(Vec::new());

        height_hash.push(header.hash());
        <BlockHeightFor<T>>::insert(&header_info.height, height_hash);

        let best_header_hash = <BestIndex<T>>::get();
        let best_header = match <BlockHeaderFor<T>>::get(&best_header_hash) {
            Some(info) => info,
            None => return Err("can't find the best header in ChainX"),
        };

        if header_info.height > best_header.height {
            //delete old header info
            let reserved = <ReservedBlock<T>>::get();
            let del = header_info.height - reserved;
            if let Some(v) = <BlockHeightFor<T>>::get(&del) {
                for h in v {
                    <BlockHeaderFor<T>>::remove(&h);
                }
            }
            <BlockHeightFor<T>>::remove(&del);

            // update confirmd status
            let params: Params = <ParamsInfo<T>>::get();
            let mut confirm_header = header_info.clone();
            for _index in 0..params.max_fork_route_preset {
                if let Some(info) =
                    <BlockHeaderFor<T>>::get(&confirm_header.header.previous_header_hash)
                {
                    confirm_header = info;
                }
            }
            <BlockHeaderFor<T>>::mutate(&confirm_header.header.hash(), |info| {
                if let Some(i) = info {
                    i.confirmed = true
                }
            });

            <BestIndex<T>>::put(header.hash());
            // TODO 遍历待确认块的 交易哈希Vec：调用[处理交易]
            <Chain<T>>::update_header(confirm_header.clone()).map_err(|e| e.info())?;
        }
        Ok(())
    }

    fn apply_push_transaction(
        tx: RelayTx,
        trustee_address: Address,
        cert_address: Address,
    ) -> Result {
        let tx_type = validate_transaction::<T>(&tx, (&trustee_address, &cert_address))?;

        //update header info
        let mut confirmed = false;
        <BlockHeaderFor<T>>::mutate(&tx.block_hash, |info| {
            if let Some(i) = info {
                i.txid.push(tx.raw.hash());

                confirmed = i.confirmed;
            }
        });
        let address = match tx_type {
            TxType::Withdraw => trustee_address,
            TxType::SendCert => cert_address,
            _ => {
                let outpoint = tx.raw.inputs[0].previous_output.clone();
                match inspect_address::<T>(&tx.previous_raw, outpoint) {
                    Some(a) => a,
                    None => return Err("inspect address failed"),
                }
            }
        };
        if !<TxFor<T>>::exists(&tx.raw.hash()) {
            <TxFor<T>>::insert(
                &tx.raw.hash(),
                TxInfo {
                    input_address: address,
                    raw_tx: tx.raw.clone(),
                },
            )
        }

        if confirmed {
            handle_tx::<T>(&tx.raw.hash()).map_err(|e| {
                runtime_io::print("handle_tx error :");
                runtime_io::print(tx.raw.hash().to_vec().as_slice());
                e
            })?;
        }

        Ok(())
    }

    //    pub fn process_btc_tx(tx: BTCTransaction, who: &T::AccountId) -> Result {
    //        handle_proposal::<T>(tx, who)
    //    }
}
