// Copyright 2019 Chainpool.

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
extern crate rustc_hex;
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

extern crate bitcrypto as crypto;
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
extern crate xr_primitives;

#[cfg(test)]
mod tests;

//mod b58;
mod blockchain;
mod header_proof;
mod tx;

use xr_primitives::generic::b58;
pub type AddrStr = XString;
use blockchain::Chain;
use chain::{BlockHeader, Transaction as BTCTransaction};
use codec::Decode;
use keys::{Address, DisplayLayout, Error as AddressError};
use primitives::compact::Compact;
use primitives::hash::H256;
use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};
use script::script::Script;
use ser::deserialize;
use system::ensure_signed;
pub use tx::RelayTx;
use tx::{
    check_withdraw_tx, create_multi_address, handle_condidate, handle_tx, inspect_address,
    sign_num, validate_transaction,
};
use xaccounts::{TrusteeAddressPair, TrusteeEntity};
use xassets::{Chain as ChainDef, ChainT};
use xr_primitives::XString;

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum TxType {
    Withdraw,
    Deposit,
    Bind,
    BindDeposit,
}

impl Default for TxType {
    fn default() -> Self {
        TxType::Deposit
    }
}

#[derive(PartialEq, Clone, Encode, Decode)]
pub struct CandidateTx<AccountId> {
    pub withdraw_id: Vec<u32>,
    pub tx: BTCTransaction,
    pub sig_status: VoteResult,
    pub sig_node: Vec<(AccountId, bool)>,
}

impl<AccountId> CandidateTx<AccountId> {
    pub fn new(
        withdraw_id: Vec<u32>,
        tx: BTCTransaction,
        sig_status: VoteResult,
        sig_node: Vec<(AccountId, bool)>,
    ) -> Self {
        CandidateTx {
            withdraw_id,
            tx,
            sig_status,
            sig_node,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum BindStatus {
    Init,
    Update,
}

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum VoteResult {
    Unfinish,
    FinishWithReject,
    FinishWithFavor,
    Invalid,
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
    pub input_address: Address,
    pub raw_tx: BTCTransaction,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
pub struct DepositCache {
    pub txid: H256,
    pub balance: u64,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeScriptInfo {
    pub hot_redeem_script: Vec<u8>,
    pub cold_redeem_script: Vec<u8>,
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

pub trait Trait:
    system::Trait + balances::Trait + timestamp::Trait + xrecords::Trait + xaccounts::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId {
        /// version, block hash, block height, prev block hash, merkle root, timestamp, nonce, wait confirm block height, wait confirm block hash
        UpdateHeader(u32, H256, u32, H256, H256, u32, u32, u32, H256),
        /// tx hash, block hash, input addr, tx type
        RecvTx(H256, H256, AddrStr, TxType),
        /// tx hash, input addr
        CertTx(H256, AddrStr),
        /// tx hash, input addr, is waiting signed original text
        WithdrawTx(H256, AddrStr, bool),
        /// tx hash, input addr, value, statue
        Deposit(H256, AddrStr, u64, bool),
        /// tx hash, input addr, account addr, bind state (init|update)
        Bind(H256, AddrStr, AccountId, BindStatus),
        /// withdrawal value, all value, cash value
        CreatProposl(u64, u64, u64),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XBridgeOfBTC {
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

        /// get IrrBlock from genesis_config
        pub ReservedBlock get(reserved) config(): u32;

        /// get IrrBlock from genesis_config
        pub IrrBlock get(irr_block) config(): u32;

        /// get BtcFee from genesis_config
        pub BtcFee get(btc_fee) config(): u64;

        pub MaxWithdrawAmount get(max_withdraw_amount) config(): u32;

        /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
        pub TxProposal get(tx_proposal): Option<CandidateTx<T::AccountId>>;

        /// tx_hash, btc value, blockhash
        pub PendingDepositMap get(pending_deposit): map Address => Option<Vec<DepositCache>>;

        pub TrusteeRedeemScript get(trustee_info): Option<TrusteeScriptInfo>;
    }
    add_extra_genesis {
        build(|storage: &mut runtime_primitives::StorageMap, _: &mut runtime_primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
            use codec::Encode;

            let (header, number): (BlockHeader, u32) = config.genesis.clone();
            let h = header.hash();

            if config.network_id == 0 && number % config.params_info.retargeting_interval != 0 {
                panic!("the blocknumber[{:}] should start from a changed difficulty block", number);
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

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn push_header(origin, header: Vec<u8>) -> Result {
            runtime_io::print("[bridge_btc] Push btc header");
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
            runtime_io::print("[bridge_btc] Push btc tx");
            ensure_signed(origin)?;
            let tx: RelayTx = Decode::decode(&mut tx.as_slice()).ok_or("Parse RelayTx err")?;
            let trustee_address = <xaccounts::TrusteeAddress<T>>::get(xassets::Chain::Bitcoin).ok_or("Should set trustee address first.")?;
            let hot_address = Address::from_layout(&trustee_address.hot_address.as_slice()).map_err(|_|"Invalid address")?;
            Self::apply_push_transaction(tx, hot_address)?;

            Ok(())
        }

        pub fn create_withdraw_tx(origin, withdraw_id: Vec<u32>, tx: Vec<u8>) -> Result {
            runtime_io::print("[bridge_btc] Push Create withdraw tx");
            let from = ensure_signed(origin)?;
            // commiter must in trustee node list
            Self::ensure_trustee_node(&from)?;
            let tx: BTCTransaction = Decode::decode(&mut tx.as_slice()).ok_or("Parse transaction err")?;
            Self::apply_create_withdraw(tx, withdraw_id)?;

            Ok(())
        }

        pub fn sign_withdraw_tx(origin, tx: Vec<u8>, vote_state: bool) -> Result {
            runtime_io::print("[bridge_btc] Push Sign withdraw tx");
            let from = ensure_signed(origin)?;
            Self::ensure_trustee_node(&from)?;
            let tx: BTCTransaction = Decode::decode(&mut tx.as_slice()).ok_or("Parse transaction err")?;
            Self::apply_sign_withdraw(from, tx, vote_state)?;

            Ok(())
        }
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"BTC";

    fn chain() -> ChainDef {
        ChainDef::Bitcoin
    }

    fn check_addr(addr: &[u8], _: &[u8]) -> Result {
        Self::verify_btc_address(addr).map_err(|_| "Verify btc addr err")?;
        Ok(())
    }
}

impl<T: Trait> Module<T> {
    pub fn verify_btc_address(data: &[u8]) -> StdResult<Address, AddressError> {
        let r = b58::from(data.to_vec()).map_err(|_| AddressError::InvalidAddress)?;
        Address::from_layout(&r)
    }

    pub fn update_trustee_addr() -> StdResult<(), AddressError> {
        let trustees = <xaccounts::TrusteeIntentions<T>>::get();
        if trustees.len() < 3 {
            return Err(AddressError::FailedKeyGeneration);
        }

        let mut hot_keys = Vec::new();
        let mut cold_keys = Vec::new();
        for trustee in trustees {
            if let Some(props) = <xaccounts::TrusteeIntentionPropertiesOf<T>>::get(&(
                trustee,
                xassets::Chain::Bitcoin,
            )) {
                match props.hot_entity {
                    TrusteeEntity::Bitcoin(pubkey) => hot_keys.push(pubkey),
                }
                match props.cold_entity {
                    TrusteeEntity::Bitcoin(pubkey) => cold_keys.push(pubkey),
                }
            }
        }
        let (hot_addr, hot_redeem) = match create_multi_address::<T>(hot_keys) {
            Some((addr, redeem)) => (addr, redeem),
            None => return Err(AddressError::InvalidAddress),
        };
        let (cold_addr, cold_redeem) = match create_multi_address::<T>(cold_keys) {
            Some((addr, redeem)) => (addr, redeem),
            None => return Err(AddressError::InvalidAddress),
        };

        let info = TrusteeScriptInfo {
            hot_redeem_script: hot_redeem.to_bytes().to_vec(),
            cold_redeem_script: cold_redeem.to_bytes().to_vec(),
        };
        <xaccounts::TrusteeAddress<T>>::insert(
            &xassets::Chain::Bitcoin,
            TrusteeAddressPair {
                hot_address: hot_addr.layout().to_vec(),
                cold_address: cold_addr.layout().to_vec(),
            },
        );
        <TrusteeRedeemScript<T>>::put(info);
        Ok(())
    }

    fn ensure_trustee_node(who: &T::AccountId) -> Result {
        let trustees = <xaccounts::TrusteeIntentions<T>>::get();
        if trustees.iter().any(|n| n == who) {
            return Ok(());
        }
        Err("Commiter no in the trustee node list")
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
            <BlockHeightFor<T>>::get(&header_info.height).unwrap_or_default();

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

            Self::deposit_event(RawEvent::UpdateHeader(
                header_info.header.version,
                header_info.header.hash(),
                header_info.height,
                header_info.header.previous_header_hash,
                header_info.header.merkle_root_hash,
                header_info.header.time,
                header_info.header.nonce,
                confirm_header.height,
                confirm_header.header.hash(),
            ));

            <BestIndex<T>>::put(header.hash());
            <Chain<T>>::update_header(confirm_header.clone()).map_err(|e| e.info())?;
        }
        Ok(())
    }

    fn apply_push_transaction(tx: RelayTx, hot_addres: Address) -> Result {
        let tx_type = validate_transaction::<T>(&tx, &hot_addres)?;

        //update header info
        let mut confirmed = false;
        <BlockHeaderFor<T>>::mutate(&tx.block_hash.clone(), |info| {
            if let Some(i) = info {
                i.txid.push(tx.raw.hash());

                confirmed = i.confirmed;
            }
        });
        let address = match tx_type {
            TxType::Withdraw => hot_addres,
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
                    input_address: address.clone(),
                    raw_tx: tx.raw.clone(),
                },
            )
        }
        let addr = address.layout().to_vec();
        Self::deposit_event(RawEvent::RecvTx(
            tx.clone().raw.hash(),
            tx.clone().block_hash,
            b58::to_base58(addr),
            tx_type,
        ));

        if confirmed {
            handle_tx::<T>(&tx.raw.hash()).map_err(|e| {
                runtime_io::print("handle_tx error :");
                runtime_io::print(tx.raw.hash().to_vec().as_slice());
                e
            })?;
        }

        Ok(())
    }

    fn apply_create_withdraw(tx: BTCTransaction, withdraw_id: Vec<u32>) -> Result {
        let trustee_address = <xaccounts::TrusteeAddress<T>>::get(xassets::Chain::Bitcoin)
            .ok_or("Should set trustee address first.")?;
        let hot_address = Address::from_layout(&trustee_address.hot_address.as_slice())
            .map_err(|_| "Invalid Address")?;
        check_withdraw_tx::<T>(tx, withdraw_id, hot_address)?;
        Ok(())
    }

    fn apply_sign_withdraw(who: T::AccountId, tx: BTCTransaction, vote_state: bool) -> Result {
        handle_condidate::<T>(tx.clone())?;
        let (sign_num, _) = sign_num::<T>();
        match <TxProposal<T>>::get() {
            Some(mut data) => {
                if data.sig_node.iter().any(|(n, _)| *n == who) {
                    return Err("Already signature transaction or reject to signature");
                }
                if !vote_state {
                    let reject_count: Vec<&(T::AccountId, bool)> = data
                        .sig_node
                        .iter()
                        .filter(|(_, vote)| *vote == false)
                        .collect();
                    data.sig_node.clone().push((who, vote_state));
                    if reject_count.len() + 1 >= sign_num {
                        runtime_io::print("Clear TxProposal");
                        <TxProposal<T>>::kill();
                        return Ok(());
                    }
                } else {
                    let script: Script = tx.inputs[0].script_sig.clone().into();
                    let (sigs, _dem) = if let Ok((sigs, dem)) = script.extract_multi_scriptsig() {
                        (sigs, dem)
                    } else {
                        return Err("No signature");
                    };
                    data.sig_node.push((who, vote_state));
                    if sigs.len() >= sign_num {
                        data.sig_status = VoteResult::FinishWithFavor;
                    }
                }

                let candidate =
                    CandidateTx::new(data.withdraw_id, tx, data.sig_status, data.sig_node);
                <TxProposal<T>>::put(candidate);
            }
            None => return Err("No pending signature transaction"),
        }

        Ok(())
    }
}
