// Copyright 2018-2019 Chainpool.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

mod assets_records;
mod header;
mod mock;
mod tests;
mod tx;
mod types;

use parity_codec::{Decode, Encode};

// Substrate
use primitives::traits::As;
use rstd::{prelude::*, result::Result as StdResult};
use support::{decl_event, decl_module, decl_storage, dispatch::Result, StorageMap, StorageValue};
use system::ensure_signed;

// ChainX
use xaccounts::TrusteeEntity;
use xassets::{Chain, ChainT, Memo, Token};
use xr_primitives::{generic::b58, traits::TrusteeForChain, XString};
use xrecords::TxState;
use xsupport::{debug, ensure_with_errorlog, error, info, warn};
#[cfg(feature = "std")]
use xsupport::{trustees, u8array_to_hex, u8array_to_string};

// light-bitcoin
use btc_chain::{BlockHeader, Transaction};
use btc_keys::{Address, DisplayLayout, Error as AddressError};
use btc_primitives::H256;
use btc_ser::{deserialize, Reader};

#[cfg(feature = "std")]
pub use self::tx::utils::hash_strip;
use self::tx::utils::{get_sig_num, get_sig_num_from_trustees, inspect_address_from_transaction};
use self::tx::{
    check_withdraw_tx, create_multi_address, detect_transaction_type, handle_tx,
    parse_and_check_signed_tx, update_trustee_vote_state, validate_transaction,
};
use self::types::{BindStatus, DepositCache, TxType};
pub use self::types::{
    BlockHeaderInfo, Params, RelayTx, TrusteeAddrInfo, TxInfo, VoteResult, WithdrawalProposal,
};

pub type AddrStr = XString;

pub trait Trait:
    system::Trait + timestamp::Trait + xrecords::Trait + xaccounts::Trait + xfee_manager::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as xassets::Trait>::Balance {
        /// version, block hash, block height, prev block hash, merkle root, timestamp, nonce, wait confirm block height, wait confirm block hash
        InsertHeader(u32, H256, u32, H256, H256, u32, u32, u32, H256),
        /// tx hash, block hash, tx type
        InsertTx(H256, H256, TxType),
        /// who, Chain, Token, apply blockheader, balance, memo, Chain Addr, chain txid, apply height, TxState
        Deposit(AccountId, Chain, Token, Balance, Memo, AddrStr, Vec<u8>, TxState),
        /// who, Chain, Token, balance,  Chain Addr
        DepositPending(AccountId, Chain, Token, Balance, AddrStr),
        /// who, withdrawal id, txid, TxState
        Withdrawal(u32, Vec<u8>, TxState),
        /// create withdraw tx, who proposal, withdrawal list id
        CreateWithdrawalProposal(AccountId, Vec<u32>),
        /// Sign withdraw tx
        UpdateSignWithdrawTx(AccountId, bool),
        /// WithdrawalFatalErr, tx hash, Proposal hash,
        WithdrawalFatalErr(Vec<u8>, Vec<u8>),
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
        pub CurrentWithdrawalProposal get(withdrawal_proposal): Option<WithdrawalProposal<T::AccountId>>;

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
        /// get BtcWithdrawalFee from genesis_config
        pub BtcWithdrawalFee get(btc_withdrawal_fee) config(): u64;
        /// max withdraw account count in bitcoin withdrawal transaction
        pub MaxWithdrawalCount get(max_withdrawal_count) config(): u32;

        // ext
        pub LastTrusteeSessionNumber get(last_trustee_session_number): u32 = 0;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn push_header(origin, header: Vec<u8>) -> Result {
            let _from = ensure_signed(origin)?;
            let header: BlockHeader = deserialize(header.as_slice()).map_err(|_| "Cannot deserialize the header vec")?;
            debug!("[push_header]|from:{:?}|header:{:?}", _from, header);

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

            let tx: Transaction = deserialize(Reader::new(tx.as_slice())).map_err(|_| "Parse transaction err")?;
            debug!("[create_withdraw_tx]|from:{:?}|withdrawal list:{:?}|tx:{:?}", from, withdrawal_id_list, tx);

            Self::apply_create_withdraw(tx, withdrawal_id_list.clone())?;

            Self::deposit_event(RawEvent::CreateWithdrawalProposal(from, withdrawal_id_list));
            Ok(())
        }

        pub fn sign_withdraw_tx(origin, tx: Option<Vec<u8>>) -> Result {
            let from = ensure_signed(origin)?;
            Self::ensure_trustee(&from)?;

            let tx = if let Some(raw_tx) = tx {
                let tx: Transaction = deserialize(Reader::new(raw_tx.as_slice())).map_err(|_| "Parse transaction err")?;
                Some(tx)
            } else {
                None
            };
            debug!("[sign_withdraw_tx]|from:{:?}|vote_tx:{:?}", from, tx);

            Self::apply_sig_withdraw(from, tx)?;
            Ok(())
        }

        pub fn remove_tx_and_proposal(txhash: Option<H256>, drop_proposal: bool) -> Result {
            if let Some(hash) = txhash {
                TxFor::<T>::remove(&hash);
                InputAddrFor::<T>::remove(&hash);
            }
            if drop_proposal {
                CurrentWithdrawalProposal::<T>::kill();
            }
            Ok(())
        }

        pub fn set_btc_withdrawal_fee(fee: T::Balance) -> Result {
            BtcWithdrawalFee::<T>::put(fee.as_() as u64);
            Ok(())
        }
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"BTC";

    fn chain() -> Chain {
        Chain::Bitcoin
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

impl<T: Trait> TrusteeForChain<T::AccountId, ()> for Module<T> {
    /// for bitcoin, it's public key, not address
    fn check_address(pubkey: &[u8]) -> Result {
        if pubkey.len() != 33 && pubkey.len() != 65 {
            return Err("Valid pubkeys are either 33 or 65 bytes.");
        }
        Ok(())
    }
    /// no support for bitcoin
    fn to_address(_: &[u8]) -> () {
        unreachable!("no support for bitcoin")
    }

    fn generate_new_trustees(
        candidates: &Vec<T::AccountId>,
    ) -> StdResult<Vec<T::AccountId>, &'static str> {
        let (trustees, _, hot_trustee_addr_info, cold_trustee_addr_info) =
            Self::try_to_generate_new_trustees(candidates)?;
        let trustees = trustees
            .into_iter()
            .map(|(accountid, _)| accountid)
            .collect::<Vec<_>>();
        info!(
            "[update_trustee_addr]|hot_addr:{:?}|cold_addr:{:?}|trustee_list:{:?}",
            hot_trustee_addr_info,
            cold_trustee_addr_info,
            trustees!(trustees)
        );

        LastTrusteeSessionNumber::<T>::put(xaccounts::Module::<T>::current_session_number(
            Chain::Bitcoin,
        ));

        xaccounts::Module::<T>::new_trustee_session(
            Chain::Bitcoin,
            trustees.clone(),
            hot_trustee_addr_info.encode(),
            cold_trustee_addr_info.encode(),
        );
        Ok(trustees)
    }
}

impl<T: Trait> Module<T> {
    /// generate trustee info, result is
    /// (trustee_list: Vec<(accountid, (hot pubkey, cold pubkey)),
    /// multisig count: (required count, total count),
    /// hot: hot_trustee_addr,
    /// cold: cold_trustee_addr)>)
    pub fn try_to_generate_new_trustees(
        candidates: &Vec<T::AccountId>,
    ) -> StdResult<
        (
            Vec<(T::AccountId, (Vec<u8>, Vec<u8>))>,
            (u32, u32),
            TrusteeAddrInfo,
            TrusteeAddrInfo,
        ),
        &'static str,
    > {
        let config = xaccounts::Module::<T>::trustee_info_config(Chain::Bitcoin);

        let mut trustee_info_list = Vec::new();
        for trustee in candidates {
            let key = (trustee.clone(), Chain::Bitcoin);
            let props =
                xaccounts::Module::<T>::trustee_intention_props_of(&key).ok_or_else(|| {
                    error!(
                        "[generate_new_trustees]|[btc] the candidate must be a trustee|who:{:?}",
                        trustee
                    );
                    "[generate_new_trustees]|[btc] the candidate must be a trustee"
                })?;

            #[allow(unreachable_patterns)]
            let hot_key = match props.hot_entity {
                TrusteeEntity::Bitcoin(pubkey) => {
                    if Self::check_address(&pubkey).is_err() {
                        error!("[generate_new_trustees]|[btc] this hot pubkey not valid!|hot pubkey:{:}", u8array_to_hex(&pubkey));
                        continue;
                    }
                    pubkey
                }
                _ => {
                    warn!("[generate_new_trustees]|[btc] this trustee do not have BITCOIN hot entity|who:{:?}", trustee);
                    continue;
                }
            };
            #[allow(unreachable_patterns)]
            let cold_key = match props.cold_entity {
                TrusteeEntity::Bitcoin(pubkey) => {
                    if Self::check_address(&pubkey).is_err() {
                        error!("[generate_new_trustees]|[btc] this hot pubkey not valid!|cold pubkey:{:}", u8array_to_hex(&pubkey));
                        continue;
                    }
                    pubkey
                }
                _ => {
                    warn!("[generate_new_trustees]|[btc] this trustee do not have BITCOIN cold entity|who:{:?}", trustee);
                    continue;
                }
            };
            trustee_info_list.push((trustee.clone(), (hot_key, cold_key)));
            // just get max trustee count
            if trustee_info_list.len() as u32 > config.max_trustee_count {
                break;
            }
        }

        if (trustee_info_list.len() as u32) < config.min_trustee_count {
            error!("[update_trustee_addr]|trustees is less than [{:}] people, can't generate trustee addr|trustees:{:?}", config.min_trustee_count, candidates);
            return Err("trustees is less than required people, can't generate trustee addr");
        }
        // // sort by AccountId
        // trustee_info_list.sort_by(|a, b| a.0.cmp(&b.0));

        let (trustees, key_pairs): (Vec<T::AccountId>, Vec<(Vec<_>, Vec<_>)>) =
            trustee_info_list.into_iter().unzip();
        let (hot_keys, cold_keys): (Vec<_>, Vec<_>) = key_pairs.into_iter().unzip();

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

        let has_duplicate = (1..hot_keys.len()).any(|i| hot_keys[i..].contains(&hot_keys[i - 1]));
        if has_duplicate {
            error!("[generate_new_trustees]|hot keys contains duplicate pubkey");
            return Err("hot keys contains duplicate pubkey");
        }
        let has_duplicate =
            (1..cold_keys.len()).any(|i| cold_keys[i..].contains(&cold_keys[i - 1]));
        if has_duplicate {
            error!("[generate_new_trustees]|cold keys contains duplicate pubkey");
            return Err("cold keys contains duplicate pubkey");
        }

        let (sig_num, trustee_num) = get_sig_num_from_trustees(trustees.len() as u32);

        let hot_trustee_addr_info: TrusteeAddrInfo =
            create_multi_address::<T>(&hot_keys, sig_num, trustee_num).ok_or_else(|| {
                error!(
                    "[update_trustee_addr]|create hot_addr err!|hot_keys:{:?}",
                    hot_keys
                );
                "create hot_addr err!"
            })?;

        let cold_trustee_addr_info: TrusteeAddrInfo =
            create_multi_address::<T>(&cold_keys, sig_num, trustee_num).ok_or_else(|| {
                error!(
                    "[update_trustee_addr]|create cold_addr err!|cold_keys:{:?}",
                    cold_keys
                );
                "create cold_addr err!"
            })?;

        let trustees_info = trustees
            .into_iter()
            .zip(hot_keys.into_iter().zip(cold_keys))
            .collect::<Vec<_>>();
        Ok((
            trustees_info,
            (sig_num, trustee_num),
            hot_trustee_addr_info,
            cold_trustee_addr_info,
        ))
    }

    pub fn verify_btc_address(data: &[u8]) -> StdResult<Address, AddressError> {
        let r = b58::from(data.to_vec()).map_err(|_| AddressError::InvalidAddress)?;
        Address::from_layout(&r)
    }

    fn ensure_trustee(who: &T::AccountId) -> Result {
        let trustee_session_info = xaccounts::Module::<T>::trustee_session_info(Chain::Bitcoin)
            .ok_or("not find current trustee session info for this chain")?;
        if trustee_session_info.trustee_list.iter().any(|n| n == who) {
            return Ok(());
        }
        error!(
            "[ensure_trustee]|Committer not in the trustee list!|who:{:?}|trustees:{:?}",
            who, trustee_session_info.trustee_list
        );
        Err("Committer not in the trustee list")
    }

    fn apply_push_header(header: BlockHeader) -> Result {
        // current should not exist
        ensure_with_errorlog!(
            Self::block_header_for(&header.hash()).is_none(),
            "Header already exists.",
            "hash:{:}",
            hash_strip(&header.hash()),
        );
        // current should exist yet
        ensure_with_errorlog!(
            Self::block_header_for(&header.previous_header_hash).is_some(),
            "Can't find previous header",
            "prev hash:{:}|current hash:{:}",
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

        debug!("[apply_push_header]|verify pass, insert to storage|height:{:}|hash:{:}block hashs for this height:{:?}",
            header_info.height,
            hash_strip(&hash),
            Self::block_hash_for(header_info.height).into_iter().map(|hash| hash_strip(&hash)).collect::<Vec<_>>()
        );

        let best_header = match Self::block_header_for(Self::best_index()) {
            Some(info) => info,
            None => return Err("can't find the best header in ChainX"),
        };

        let (confirm_hash, confirm_height) = if header_info.height > best_header.height {
            header::remove_unused_headers::<T>(&header_info);

            let (confirm_hash, confirm_height) = header::update_confirmed_header::<T>(&header_info);
            info!(
                "[apply_push_header]|update to new height|height:{:}|hash:{:}",
                header_info.height,
                hash_strip(&hash),
            );
            // change new best index
            BestIndex::<T>::put(hash);
            (confirm_hash, confirm_height)
        } else {
            info!("[apply_push_header]|best index larger than this height|best height:{:}|this height{:}",
                best_header.height,
                header_info.height
            );
            (Default::default(), Default::default())
        };
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

        Ok(())
    }

    fn apply_push_transaction(tx: RelayTx) -> Result {
        let tx_hash = tx.raw.hash();
        let mut header_info = Module::<T>::block_header_for(&tx.block_hash).ok_or_else(|| {
            error!(
                "[apply_push_transaction]|tx's block header must exist before|block_hash:{:}",
                tx.block_hash
            );
            "tx's block header must exist before"
        })?;
        let merkle_root = header_info.header.merkle_root_hash;
        // verify, check merkle proof
        validate_transaction::<T>(&tx, merkle_root)?;

        let height = header_info.height;
        let confirmed = header_info.confirmed;
        // notice same tx may belong to different forked block, after check merkle proof, it's all valid
        if !header_info.txid_list.contains(&tx_hash) {
            header_info.txid_list.push(tx_hash.clone());
            // modify block info storage
            BlockHeaderFor::<T>::insert(&tx.block_hash, header_info);
        } else {
            // not pass check! this tx has already been inserted to this block
            error!("[apply_push_transaction]|this block already has this tx|block_hash:{:}|tx_hash:{:}|tx_list:{:?}", tx.block_hash, hash_strip(&tx_hash), header_info.txid_list);
            return Err("this block already has this tx");
        }

        // same tx may in different forked block, thus, just modify different forked block txlist, and the tx only insert once
        // so when the tx is existed, return tx_type, else set `TxFor` and `InputAddrFor` storage, return tx_type
        // get tx_type
        let (tx_type, _existed) = match Self::tx_for(&tx_hash) {
            None => {
                let tx_type = detect_transaction_type::<T>(&tx)?;
                if tx_type == TxType::Irrelevance {
                    warn!("[apply_push_transaction]|this tx is not related to any important addr, maybe an irrelevance tx, drop it|relay_tx:{:?}|block_height:{:}", tx, height);
                    return Err("this tx is not related to any important addr, maybe an irrelevance tx, drop it");
                }
                // parse first input addr, may delete when only use opreturn to get accountid
                // only deposit tx store prev input tx's addr, for deposit to lookup related accountid
                if tx_type == TxType::Deposit {
                    let outpoint = &tx.raw.inputs[0].previous_output;
                    let input_addr =
                        inspect_address_from_transaction::<T>(&tx.previous_raw, outpoint)
                            .expect("when deposit, the first input must could parse an addr; qed");

                    debug!(
                        "[apply_push_transaction]|deposit input addr|txhash:{:}|addr:{:}",
                        hash_strip(&tx_hash),
                        u8array_to_string(&b58::to_base58(input_addr.layout().to_vec())),
                    );
                    InputAddrFor::<T>::insert(&tx_hash, input_addr)
                }
                // set tx into storage
                TxFor::<T>::insert(
                    &tx_hash,
                    TxInfo {
                        raw_tx: tx.raw.clone(),
                        tx_type,
                        height,
                        done: false,
                    },
                );
                (tx_type, false)
            }
            Some(tx_info) => (tx_info.tx_type, true),
        };

        debug!(
            "[apply_push_transaction]|verify pass|txhash:{:}|is existed:{:}|tx type:{:?}|block_hash:{:}|height:{:}|confirmed:{:}",
            hash_strip(&tx_hash),
            _existed,
            tx_type,
            hash_strip(&tx.block_hash),
            height,
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
                error!("Handle tx error: {:}", hash_strip(&tx_hash));
                e
            })?;
        }

        Ok(())
    }

    fn apply_create_withdraw(tx: Transaction, withdrawal_id_list: Vec<u32>) -> Result {
        let withdraw_amount = Self::max_withdrawal_count();
        if withdrawal_id_list.len() > withdraw_amount as usize {
            error!("[apply_create_withdraw]|Exceeding the maximum withdrawal amount|current list len:{:}|max:{:}", withdrawal_id_list.len(), withdraw_amount);
            return Err("Exceeding the maximum withdrawal amount");
        }
        // remove duplicate
        let mut withdrawal_id_list = withdrawal_id_list;
        withdrawal_id_list.sort();
        withdrawal_id_list.dedup();

        check_withdraw_tx::<T>(&tx, &withdrawal_id_list)?;

        info!(
            "[apply_create_withdraw]|create new withdraw|withdrawal idlist:{:?}",
            withdrawal_id_list
        );

        // log event
        for id in withdrawal_id_list.iter() {
            Self::deposit_event(RawEvent::Withdrawal(*id, Vec::new(), TxState::Signing));
        }

        let candidate =
            WithdrawalProposal::new(VoteResult::Unfinish, withdrawal_id_list, tx, Vec::new());
        CurrentWithdrawalProposal::<T>::put(candidate);
        info!("[apply_create_withdraw]|Through the legality check of withdrawal");
        Ok(())
    }

    fn apply_sig_withdraw(who: T::AccountId, tx: Option<Transaction>) -> Result {
        let mut proposal: WithdrawalProposal<T::AccountId> =
            Self::withdrawal_proposal().ok_or("No transactions waiting for signature")?;

        if proposal.sig_state == VoteResult::Finish {
            error!("[apply_sig_withdraw]|proposal is on FINISH state, can't sign for this proposal|proposal：{:?}", proposal);
            return Err("proposal is on FINISH state, can't sign for this proposal");
        }

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

                if sigs.len() as u32 >= sig_num {
                    info!("[apply_sig_withdraw]Signature completed: {:}", sigs.len());
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
                    .count() as u32;
                if reject_count >= sig_num {
                    info!(
                        "[apply_sig_withdraw]|{:}/{:} opposition, clear withdrawal propoal",
                        reject_count, sig_num
                    );
                    CurrentWithdrawalProposal::<T>::kill();

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

        CurrentWithdrawalProposal::<T>::put(proposal);
        Ok(())
    }
}
