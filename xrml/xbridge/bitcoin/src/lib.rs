// Copyright 2019 Chainpool.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

mod assets_records;
mod header;
mod tx;
mod types;

#[cfg(test)]
mod tests;

use parity_codec as codec;

use sr_primitives as runtime_primitives;
use sr_std as rstd;

use srml_balances as balances;
use srml_support as support;
use srml_system as system;
use srml_timestamp as timestamp;

use xr_primitives::{generic::b58, XString};
use xrml_xaccounts as xaccounts;
use xrml_xassets_assets as xassets;
use xrml_xassets_records as xrecords;
use xrml_xsupport as xsupport;

// bitcoin-rust
use bitcrypto;
use bitcrypto as crypto;
use chain as btc_chain;
use keys as btc_keys;
use merkle;
use primitives as btc_primitives;
use script as btc_script;
use serialization as btc_ser;

//use blockchain::Chain;
use crate::btc_chain::{BlockHeader, Transaction};
use crate::btc_keys::{Address, DisplayLayout, Error as AddressError};
use crate::btc_primitives::hash::H256;
use crate::btc_ser::{deserialize, Reader};
use crate::codec::Decode;

use crate::rstd::prelude::*;
use crate::rstd::result::Result as StdResult;
use crate::runtime_primitives::traits::As;
use crate::support::{
    decl_event, decl_module, decl_storage, dispatch::Result, StorageMap, StorageValue,
};
use crate::system::ensure_signed;
use crate::xaccounts::{TrusteeAddressPair, TrusteeEntity};
use crate::xassets::{Chain as ChainDef, ChainT, Memo, Token};
use crate::xrecords::TxState;

#[cfg(feature = "std")]
pub use tx::utils::hash_strip;
use tx::utils::inspect_address_from_transaction;
use tx::{
    check_withdraw_tx, create_multi_address, get_sig_num, handle_tx, parse_and_check_signed_tx,
    remove_unused_tx, update_trustee_vote_state, validate_transaction,
};
use types::{BindStatus, DepositCache, TxType};
pub use types::{
    BlockHeaderInfo, CandidateTx, Params, RelayTx, TrusteeScriptInfo, TxInfo, VoteResult,
};

use crate::xsupport::{debug, ensure_with_errorlog, error, info};
#[cfg(feature = "std")]
use crate::xsupport::{u8array_to_hex, u8array_to_string};

pub type AddrStr = XString;

pub trait Trait:
    system::Trait + balances::Trait + timestamp::Trait + xrecords::Trait + xaccounts::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as balances::Trait>::Balance {
        /// version, block hash, block height, prev block hash, merkle root, timestamp, nonce, wait confirm block height, wait confirm block hash
        InsertHeader(u32, H256, u32, H256, H256, u32, u32, u32, H256),
        /// tx hash, block hash, tx type
        InsertTx(H256, H256, TxType),
        /// who, Chain, Token, apply blockheader, balance, memo, Chain Addr, chain txid, apply height, TxState
        Deposit(AccountId, ChainDef, Token, Balance, Memo, AddrStr, Vec<u8>, TxState),
        /// who, Chain, Token, balance,  Chain Addr
        DepositPending(AccountId, ChainDef, Token, Balance, AddrStr),
        /// who, withdrawal id, txid, TxState
        Withdrawal(u32, Vec<u8>, TxState),
        /// create withdraw tx, who proposal, withdrawal list id
        CreateWithdrawTx(AccountId, Vec<u32>),
        /// Sign withdraw tx
        UpdateSignWithdrawTx(AccountId, bool),
        /// NeedDropWithdrawTx, Proposal hash, tx hash
        NeedDropWithdrawTx(Vec<u8>, Vec<u8>),
        /// reject_count, sum_count, withdrawal id list
        DropWithdrawTx(u32, u32, Vec<u32>),
        /// tx hash, input addr, account addr, bind state (init|update)
        Bind(H256, AddrStr, AccountId, BindStatus),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XBridgeOfBTC {
        /// get bestheader
        pub BestIndex get(best_index): H256;
        /// block hash list for a height
        pub BlockHashFor get(block_hash_for): map u32 => Vec<H256>;
        /// all valid blockheader (include orphan blockheader)
        pub BlockHeaderFor get(block_header_for): map H256 => Option<BlockHeaderInfo>;
        /// tx info for txhash
        pub TxFor get(tx_for): map H256 => Option<TxInfo>;
        /// tx first input addr for this tx
        pub InputAddrFor get(input_addr_for): map H256 => Option<Address>;

        /// unclaim deposit info, addr => tx_hash, btc value, blockhash
        pub PendingDepositMap get(pending_deposit): map Address => Option<Vec<DepositCache>>;
        /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
        pub WithdrawalProposal get(withdrawal_proposal): Option<CandidateTx<T::AccountId>>;
        pub WithdrawalFatalErr get(withdrawal_fatal_err): bool = false;

        /// get GenesisInfo (header, height)
        pub GenesisInfo get(genesis_info) config(genesis): (BlockHeader, u32);
        /// get ParamsInfo from genesis_config
        pub ParamsInfo get(params_info) config(): Params;
        ///  NetworkId for testnet or mainnet
        pub NetworkId get(network_id): u32;
        /// reserved count for block
        pub ReservedBlock get(reserved_block) config(): u32;
        /// get ConfirmationNumber from genesis_config
        pub ConfirmationNumber get(confirmation_number) config(): u32;
        /// trustee script
        pub TrusteeRedeemScript get(trustee_info): Option<TrusteeScriptInfo>;
        /// get BtcWithdrawalFee from genesis_config
        pub BtcWithdrawalFee get(btc_withdrawal_fee) config(): u64;
        /// max withdraw account count in bitcoin withdrawal transaction
        pub MaxWithdrawalCount get(max_withdrawal_count) config(): u32;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn push_header(origin, header: Vec<u8>) -> Result {
            let _from = ensure_signed(origin)?;
            let header: BlockHeader = deserialize(header.as_slice()).map_err(|_| "Cannot deserialize the header vec")?;
            debug!("[push_header]|from:{:}|header:{:?}", _from, header);

            Self::apply_push_header(header)?;
            Ok(())
        }

        pub fn push_transaction(origin, tx: Vec<u8>) -> Result {
            let _from = ensure_signed(origin)?;
            let tx: RelayTx = Decode::decode(&mut tx.as_slice()).ok_or("Parse RelayTx err")?;
            debug!("[push_transaction]|from:{:?}|relay_tx:{:?}", _from, tx);

            Self::apply_push_transaction(tx)?;
            Ok(())
        }

        pub fn create_withdraw_tx(origin, withdrawal_id_list: Vec<u32>, tx: Vec<u8>) -> Result {
            let from = ensure_signed(origin)?;
            // commiter must in trustee list
            Self::ensure_trustee(&from)?;

            ensure_with_errorlog!(
                Self::withdrawal_fatal_err() == false,
                "there is a fatal error for current proposal, please use root to call [fix_withdrawal_err] to fix it",
                "proposal:{:?}",
                Self::withdrawal_proposal(),
            );

            let tx: Transaction = deserialize(Reader::new(tx.as_slice())).map_err(|_| "Parse transaction err")?;
            debug!("[create_withdraw_tx]|from:{:}|withdrawal list:{:?}|tx:{:?}", from, withdrawal_id_list, tx);

            Self::apply_create_withdraw(tx, withdrawal_id_list.clone())?;

            Self::deposit_event(RawEvent::CreateWithdrawTx(from, withdrawal_id_list));
            Ok(())
        }

        pub fn sign_withdraw_tx(origin, tx: Option<Vec<u8>>) -> Result {
            let from = ensure_signed(origin)?;
            Self::ensure_trustee(&from)?;

            ensure_with_errorlog!(
                Self::withdrawal_fatal_err() == false,
                "there is a fatal error for current proposal, please use root to call [fix_withdrawal_err] to fix it",
                "proposal:{:?}",
                Self::withdrawal_proposal(),
            );

            let tx = if let Some(raw_tx) = tx {
                let tx: Transaction = deserialize(Reader::new(raw_tx.as_slice())).map_err(|_| "Parse transaction err")?;
                Some(tx)
            } else {
                None
            };
            debug!("[sign_withdraw_tx]|from:{:}|vote_tx:{:?}", from, tx);

            Self::apply_sig_withdraw(from, tx)?;
            Ok(())
        }

        /// real btc has been withdrawed, but not equal to proposal, so the proposal and tx not remove
        /// from storage, and the withdrawal lock not release
        pub fn fix_withdrawal_err(txid: H256, withdrawal_idlist: Vec<(u32, bool)>, drop_proposal: bool) -> Result {
            for (withdrawal_id, success) in withdrawal_idlist {
                match xrecords::Module::<T>::withdrawal_finish(withdrawal_id, success) {
                    Ok(_) => {
                        info!("[withdraw]|ID of withdrawal completion: {:}", withdrawal_id);
                    }
                    Err(_e) => {
                        error!("[withdraw]|ID of withdrawal ERROR! {:}, reason:{:}, please use root to fix it", withdrawal_id, _e);
                    }
                }
            }
            if drop_proposal {
                WithdrawalProposal::<T>::kill();
            }
            remove_unused_tx::<T>(&txid);
            WithdrawalFatalErr::<T>::put(false);
            Ok(())
        }

        pub fn fix_deposit_err(txid: H256) -> Result {
            let tx_info = Self::tx_for(&txid).ok_or("not find tx for this txid")?;
            ensure_with_errorlog!(
                tx_info.tx_type == TxType::Deposit,
                "must be a deposit tx, not allow withdrawal",
                "tx:{:?}", tx_info
            );

            handle_tx::<T>(&txid)
        }

        pub fn set_btc_withdrawal_fee(fee: T::Balance) -> Result {
            BtcWithdrawalFee::<T>::put(fee.as_() as u64);
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
        // this addr is base58 addr
        let _ = Self::verify_btc_address(addr)
            .map_err(|_| "Verify btc addr err")
            .map_err(|e| {
                error!(
                    "[verify_btc_address]|failed, source addr is:{:?}",
                    u8array_to_string(addr)
                );
                e
            })?;
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
            error!("[update_trustee_addr]|trustees is less than 3 people, can't generate trustee addr. trustees:{:?}", trustees);
            return Err(AddressError::FailedKeyGeneration);
        }

        let mut hot_keys = Vec::new();
        let mut cold_keys = Vec::new();
        for trustee in trustees {
            if let Some(props) =
                <xaccounts::TrusteeIntentionPropertiesOf<T>>::get(&(trustee, ChainDef::Bitcoin))
            {
                match props.hot_entity {
                    TrusteeEntity::Bitcoin(pubkey) => hot_keys.push(pubkey),
                }
                match props.cold_entity {
                    TrusteeEntity::Bitcoin(pubkey) => cold_keys.push(pubkey),
                }
            }
        }
        hot_keys.sort();
        cold_keys.sort();

        info!(
            "[update_trustee_addr]|hot_keys:{:?}|cold_keys:{:?}",
            hot_keys
                .iter()
                .map(|s| u8array_to_hex(s))
                .collect::<Vec<_>>(),
            cold_keys
                .iter()
                .map(|s| u8array_to_hex(s))
                .collect::<Vec<_>>(),
        );

        let (hot_addr, hot_redeem) = match create_multi_address::<T>(hot_keys) {
            Some((addr, redeem)) => (addr, redeem),
            None => {
                error!("[update_trustee_addr]|create hot_addr err!");
                return Err(AddressError::InvalidAddress);
            }
        };
        let (cold_addr, cold_redeem) = match create_multi_address::<T>(cold_keys) {
            Some((addr, redeem)) => (addr, redeem),
            None => {
                error!("[update_trustee_addr]|create cold_addr err!");
                return Err(AddressError::InvalidAddress);
            }
        };

        info!(
            "[update_trustee_addr]|hot_addr:{:?}|hot_redeem:{:?}|cold_addr:{:?}|cold_redeem:{:?}",
            hot_addr, hot_redeem, cold_addr, cold_redeem
        );

        let info = TrusteeScriptInfo {
            hot_redeem_script: hot_redeem.to_bytes().to_vec(),
            cold_redeem_script: cold_redeem.to_bytes().to_vec(),
        };
        // TODO delay put
        <xaccounts::TrusteeAddress<T>>::insert(
            &ChainDef::Bitcoin,
            TrusteeAddressPair {
                hot_address: hot_addr.layout().to_vec(),
                cold_address: cold_addr.layout().to_vec(),
            },
        );

        <TrusteeRedeemScript<T>>::put(info);
        Ok(())
    }

    fn ensure_trustee(who: &T::AccountId) -> Result {
        let trustees = <xaccounts::TrusteeIntentions<T>>::get();
        if trustees.iter().any(|n| n == who) {
            return Ok(());
        }
        error!(
            "[ensure_trustee]|Committer not in the trustee list!|who:{:}|trustees:{:?}",
            who, trustees
        );
        Err("Committer not in the trustee list")
    }

    fn apply_push_header(header: BlockHeader) -> Result {
        // current should not exist
        ensure_with_errorlog!(
            Self::block_header_for(&header.hash()).is_none(),
            "Header already exists.",
            "hash:{:}...",
            hash_strip(&header.hash()),
        );
        // current should exist yet
        ensure_with_errorlog!(
            Self::block_header_for(&header.previous_header_hash).is_some(),
            "Can't find previous header",
            "prev hash:{:}...|current hash:{:}",
            hash_strip(&header.previous_header_hash),
            hash_strip(&header.hash()),
        );

        // convert btc header to self header info
        let header_info: BlockHeaderInfo =
            header::check_prev_and_convert::<T>(header).map_err(|e| e.info())?;
        // check
        let c = header::HeaderVerifier::new::<T>(&header_info.header, header_info.height)
            .map_err(|e| e.info())?;
        c.check::<T>()?;

        // insert into storage
        let hash = header_info.header.hash();
        // insert valid header into storage
        BlockHeaderFor::<T>::insert(&hash, header_info.clone());
        BlockHashFor::<T>::mutate(header_info.height, |v| {
            if !v.contains(&hash) {
                v.push(hash.clone());
            }
        });

        debug!("[apply_push_header]|verify pass, insert to storage|height:{:}|hash:{:}...|block hashs for this height:{:?}",
            header_info.height,
            hash_strip(&hash),
            Self::block_hash_for(header_info.height).into_iter().map(|hash| hash_strip(&hash)).collect::<Vec<_>>()
        );

        let best_header = match Self::block_header_for(Self::best_index()) {
            Some(info) => info,
            None => return Err("can't find the best header in ChainX"),
        };

        if header_info.height > best_header.height {
            header::remove_unused_headers::<T>(&header_info);

            let (confirm_hash, confirm_height) = header::update_confirmed_header::<T>(&header_info);
            Self::deposit_event(RawEvent::InsertHeader(
                header_info.header.version,
                header_info.header.hash(),
                header_info.height,
                header_info.header.previous_header_hash,
                header_info.header.merkle_root_hash,
                header_info.header.time,
                header_info.header.nonce,
                confirm_height,
                confirm_hash,
            ));

            info!(
                "[apply_push_header]|update to new height|height:{:}|hash:{:}...",
                header_info.height,
                hash_strip(&hash),
            );
            // change new best index
            BestIndex::<T>::put(hash);
        } else {
            info!("[apply_push_header]|best index larger than this height|best height:{:}|this height{:}",
                best_header.height,
                header_info.height
            );
        }
        Ok(())
    }

    fn apply_push_transaction(tx: RelayTx) -> Result {
        let tx_hash = tx.raw.hash();
        ensure_with_errorlog!(
            Self::tx_for(&tx_hash).is_none(),
            "tx already exists.",
            "hash:{:}...",
            hash_strip(&tx_hash)
        );

        // verify
        validate_transaction::<T>(&tx)?;
        // judge tx type
        let tx_type = tx::detect_transaction_type::<T>(&tx)?;
        // set to storage
        let mut confirmed = false;
        BlockHeaderFor::<T>::mutate(&tx.block_hash, |info| {
            if let Some(header_info) = info {
                if !header_info.txid_list.contains(&tx_hash) {
                    header_info.txid_list.push(tx_hash.clone());
                }
                confirmed = header_info.confirmed;
            }
        });

        // parse first input addr, may delete when only use opreturn to get accountid
        // only deposit tx store prev input tx's addr, for deposit to lookup related accountid
        if tx_type == TxType::Deposit {
            let outpoint = &tx.raw.inputs[0].previous_output;
            if let Some(input_addr) =
                inspect_address_from_transaction::<T>(&tx.previous_raw, outpoint)
            {
                debug!(
                    "[apply_push_transaction]|deposit input addr|txhash:{:}|addr:{:}",
                    hash_strip(&tx_hash),
                    u8array_to_string(&b58::to_base58(input_addr.layout().to_vec())),
                );
                InputAddrFor::<T>::insert(&tx_hash, input_addr)
            } else {
                assert!(
                    false,
                    "when deposit, the first input must could parse an addr"
                );
            }
        }
        TxFor::<T>::insert(
            &tx_hash,
            TxInfo {
                raw_tx: tx.raw.clone(),
                tx_type,
            },
        );

        debug!(
            "[apply_push_transaction]|verify pass|txhash:{:}|tx type:{:?}|confirmed:{:}",
            hash_strip(&tx_hash),
            tx_type,
            confirmed
        );

        // log event
        Self::deposit_event(RawEvent::InsertTx(
            tx_hash.clone(),
            tx.block_hash.clone(),
            tx_type,
        ));
        // if confirm, handle this tx for deposit or withdrawal
        if confirmed {
            handle_tx::<T>(&tx_hash).map_err(|e| {
                error!("Handle tx error: {:}...", hash_strip(&tx_hash));
                e
            })?;
        }

        Ok(())
    }

    fn apply_create_withdraw(tx: Transaction, withdrawal_id_list: Vec<u32>) -> Result {
        let withdraw_amount = Self::max_withdrawal_count();
        if withdrawal_id_list.len() > withdraw_amount as usize {
            return Err("Exceeding the maximum withdrawal amount");
        }
        // remove duplicate
        let mut withdrawal_id_list = withdrawal_id_list;
        withdrawal_id_list.sort();
        let mut list = vec![];
        let mut last_see = 0_u32;
        for i in withdrawal_id_list {
            if i != last_see {
                last_see = i;
                list.push(last_see);
            }
        }
        let withdrawal_id_list = list;

        check_withdraw_tx::<T>(&tx, &withdrawal_id_list)?;

        info!(
            "[apply_create_withdraw]|create new withdraw|withdrawal idlist:{:?}",
            withdrawal_id_list
        );

        // log event
        for id in withdrawal_id_list.iter() {
            Self::deposit_event(RawEvent::Withdrawal(*id, Vec::new(), TxState::Signing));
        }

        let candidate = CandidateTx::new(VoteResult::Unfinish, withdrawal_id_list, tx, Vec::new());
        WithdrawalProposal::<T>::put(candidate);
        info!("Through the legality check of withdrawal");
        Ok(())
    }

    fn apply_sig_withdraw(who: T::AccountId, tx: Option<Transaction>) -> Result {
        let mut proposal: CandidateTx<T::AccountId> =
            Self::withdrawal_proposal().ok_or("No transactions waiting for signature")?;

        let (sig_num, _) = get_sig_num::<T>();
        match tx {
            Some(tx) => {
                // sign
                // check first and get signatures from commit transaction
                let sigs = parse_and_check_signed_tx::<T>(&tx)?;

                if sigs.len() <= proposal.trustee_list.len() {
                    error!(
                        "[apply_sig_withdraw]|tx sigs len:{:}|proposal trustee len:{:}",
                        sigs.len(),
                        proposal.trustee_list.len()
                    );
                    return Err("Need to sign on the latest signature results");
                }

                update_trustee_vote_state::<T>(true, &who, &mut proposal.trustee_list);

                if sigs.len() >= sig_num {
                    info!("Signature completed: {:}", sigs.len());
                    proposal.sig_state = VoteResult::Finish;

                    // log event
                    for id in proposal.withdrawal_id_list.iter() {
                        Self::deposit_event(RawEvent::Withdrawal(
                            *id,
                            Vec::new(),
                            TxState::Broadcasting,
                        ));
                    }
                } else {
                    proposal.sig_state = VoteResult::Unfinish;
                }
                // update tx
                proposal.tx = tx;
            }
            None => {
                // reject
                update_trustee_vote_state::<T>(false, &who, &mut proposal.trustee_list);

                let reject_count = proposal
                    .trustee_list
                    .iter()
                    .filter(|(_, vote)| *vote == false)
                    .count();
                if reject_count >= sig_num {
                    info!(
                        "[apply_sig_withdraw]|{:}/{:} opposition, clear withdrawal propoal",
                        reject_count, sig_num
                    );
                    WithdrawalProposal::<T>::kill();

                    // log event
                    for id in proposal.withdrawal_id_list.iter() {
                        Self::deposit_event(RawEvent::Withdrawal(
                            *id,
                            Vec::new(),
                            TxState::Applying,
                        ));
                    }

                    Self::deposit_event(RawEvent::DropWithdrawTx(
                        reject_count as u32,
                        sig_num as u32,
                        proposal.withdrawal_id_list.clone(),
                    ));
                    return Ok(());
                }
            }
        }

        info!(
            "[apply_sig_withdraw]|current sig|state:{:?}|trustee vote:{:?}",
            proposal.sig_state, proposal.trustee_list
        );

        WithdrawalProposal::<T>::put(proposal);
        Ok(())
    }
}
