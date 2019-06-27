// Copyright 2018-2019 Chainpool.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

mod assets_records;
mod header;
mod mock;
mod tests;
mod tx;
mod types;

use parity_codec::Decode;

// Substrate
use primitives::traits::As;
use rstd::{prelude::*, result};
use support::{decl_event, decl_module, decl_storage, dispatch::Result, StorageMap, StorageValue};
use system::ensure_signed;

// ChainX
use xassets::{Chain, ChainT, Memo, Token};
use xbridge_common::{
    traits::{CrossChainBinding, Extractable, TrusteeForChain, TrusteeMultiSig, TrusteeSession},
    types::{TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo},
    utils::two_thirds_unsafe,
};
use xr_primitives::{generic::b58, AddrStr};
use xrecords::{ApplicationState, TxState};
use xsupport::{debug, ensure_with_errorlog, error, info, warn};
#[cfg(feature = "std")]
use xsupport::{trustees, u8array_to_string};

// light-bitcoin
use btc_chain::{BlockHeader, Transaction};
use btc_keys::{Address as BitcoinAddress, DisplayLayout, Error as AddressError, Public};
use btc_primitives::H256;
pub use btc_primitives::H264;
use btc_ser::{deserialize, Reader};

use self::tx::handler::remove_pending_deposit;
use self::tx::utils::{get_sig_num, get_trustee_address_pair, trustee_session};
use self::tx::{
    check_withdraw_tx, create_multi_address, detect_transaction_type, handle_tx,
    insert_trustee_vote_state, parse_and_check_signed_tx, validate_transaction,
};
pub use self::types::{
    BlockHeaderInfo, Params, RelayTx, TrusteeAddrInfo, TxInfo, VoteResult, WithdrawalProposal,
};
use self::types::{DepositCache, TxType};

pub trait Trait:
    system::Trait + timestamp::Trait + xsystem::Trait + xrecords::Trait + xfee_manager::Trait
{
    type AccountExtractor: Extractable<Self::AccountId>;
    type TrusteeSessionProvider: TrusteeSession<Self::AccountId, TrusteeAddrInfo>;
    type TrusteeMultiSigProvider: TrusteeMultiSig<Self::AccountId>;
    type CrossChainProvider: CrossChainBinding<Self::AccountId, BitcoinAddress>;
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as xassets::Trait>::Balance {
        /// version, block hash, block height, prev block hash, merkle root, timestamp, nonce, wait confirmed block height, wait confirmed block hash
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
        SignWithdrawalProposal(AccountId, bool),
        /// WithdrawalFatalErr, tx hash, Proposal hash,
        WithdrawalFatalErr(Vec<u8>, Vec<u8>),
        /// reject_count, sum_count, withdrawal id list
        DropWithdrawalProposal(u32, u32, Vec<u32>),
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
        /// mark tx has been handled, in case re-handle this tx
        /// do not need to remove after this tx is removed from ChainX
        pub TxMarkFor get(tx_mark_for): map H256 => Option<()>;
        /// tx first input addr for this tx
        pub InputAddrFor get(input_addr_for): map H256 => Option<BitcoinAddress>;

        /// unclaim deposit info, addr => tx_hash, btc value, blockhash
        pub PendingDepositMap get(pending_deposit): map BitcoinAddress => Option<Vec<DepositCache>>;
        /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
        pub CurrentWithdrawalProposal get(withdrawal_proposal): Option<WithdrawalProposal<T::AccountId>>;

        /// get GenesisInfo (header, height)
        pub GenesisInfo get(genesis_info) config(genesis): (BlockHeader, u32);
        /// get ParamsInfo from genesis_config
        pub ParamsInfo get(params_info) config(): Params;
        ///  NetworkId for testnet or mainnet
        pub NetworkId get(network_id) config(): u32;
        /// reserved count for block
        pub ReservedBlock get(reserved_block) config(): u32;
        /// get ConfirmationNumber from genesis_config
        pub ConfirmationNumber get(confirmation_number) config(): u32;
        /// get BtcWithdrawalFee from genesis_config
        pub BtcWithdrawalFee get(btc_withdrawal_fee) config(): u64;
        /// max withdraw account count in bitcoin withdrawal transaction
        pub MaxWithdrawalCount get(max_withdrawal_count) config(): u32;
    }
    add_extra_genesis {
        config(genesis_hash): H256;
        build(|storage: &mut primitives::StorageOverlay, _: &mut primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
            use runtime_io::with_externalities;
            use substrate_primitives::Blake2Hasher;
            use primitives::StorageOverlay;
            use support::{StorageMap, StorageValue};

            let (genesis_header, number): (BlockHeader, u32) = config.genesis.clone();
            if config.network_id == 0 && number % config.params_info.retargeting_interval() != 0 {
                panic!("the blocknumber[{:}] should start from a changed difficulty block", number);
            }

            let genesis_hash = genesis_header.hash();

            if genesis_hash != config.genesis_hash {
                panic!("the genesis block not much the genesis_hash!|genesis_block's hash:{:?}|config genesis_hash:{:?}", genesis_hash, config.genesis_hash);
            }

            let header_info = BlockHeaderInfo {
                header: genesis_header,
                height: number,
                confirmed: true,
                txid_list: [].to_vec(),
            };

            let s = storage.clone().build_storage().unwrap().0;
            let mut init: runtime_io::TestExternalities<Blake2Hasher> = s.into();
            with_externalities(&mut init, || {
                BlockHeaderFor::<T>::insert(&genesis_hash, header_info.clone());
                BlockHashFor::<T>::insert(&header_info.height, vec![genesis_hash.clone()]);

                BestIndex::<T>::put(genesis_hash);

                Module::<T>::deposit_event(RawEvent::InsertHeader(
                    header_info.header.version,
                    header_info.header.hash(),
                    header_info.height,
                    header_info.header.previous_header_hash,
                    header_info.header.merkle_root_hash,
                    header_info.header.time,
                    header_info.header.nonce,
                    header_info.height,
                    genesis_hash,
                ));
            });
            let init: StorageOverlay = init.into();
            storage.extend(init);
        });
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

            Self::apply_create_withdraw(from, tx, withdrawal_id_list.clone())?;
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

        pub fn remove_pending(addr: BitcoinAddress, who: Option<T::AccountId>) -> Result {
            if let Some(w) = who {
                remove_pending_deposit::<T>(&addr, &w);
            } else {
                info!("[remove_pending]|release pending deposit directly, not deposit to someone|addr:{:?}", addr);
                PendingDepositMap::<T>::remove(&addr);
            }
            Ok(())
        }

        pub fn set_btc_withdrawal_fee(fee: T::Balance) -> Result {
            BtcWithdrawalFee::<T>::put(fee.as_() as u64);
            Ok(())
        }

        pub fn set_btc_withdrawal_fee_by_trustees(origin, fee: T::Balance) -> Result {
            let from = ensure_signed(origin)?;
            T::TrusteeMultiSigProvider::check_multisig(&from)?;

            Self::set_btc_withdrawal_fee(fee)
        }

        pub fn fix_withdrawal_state_by_trustees(origin, withdrawal_id: u32, state: ApplicationState) -> Result {
            let from = ensure_signed(origin)?;
            T::TrusteeMultiSigProvider::check_multisig(&from)?;
            xrecords::Module::<T>::fix_withdrawal_state_by_trustees(Chain::Bitcoin, withdrawal_id, state)
        }

        pub fn remove_pending_by_trustees(origin, addr: BitcoinAddress, who: Option<T::AccountId>) -> Result {
            let from = ensure_signed(origin)?;
            T::TrusteeMultiSigProvider::check_multisig(&from)?;
            Self::remove_pending(addr, who)
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
        let address = Self::verify_btc_address(addr)
            .map_err(|_| "Verify btc addr err")
            .map_err(|e| {
                error!(
                    "[verify_btc_address]|failed, source addr is:{:?}",
                    u8array_to_string(addr)
                );
                e
            })?;

        let (hot_addr, cold_addr) = get_trustee_address_pair::<T>()?;
        if address == hot_addr || address == cold_addr {
            return Err("current addr is equal to hot or cold trustee addr");
        }

        Ok(())
    }
}

fn check_keys(keys: &[Public]) -> Result {
    let has_duplicate = (1..keys.len()).any(|i| keys[i..].contains(&keys[i - 1]));
    if has_duplicate {
        error!("[generate_new_trustees]|keys contains duplicate pubkey");
        return Err("keys contains duplicate pubkey");
    }
    if keys.iter().any(|public: &Public| {
        if let Public::Normal(_) = public {
            true
        } else {
            false
        }
    }) {
        return Err("unexpect! all keys(bitcoin Public) should be compressed");
    }
    Ok(())
}

impl<T: Trait> TrusteeForChain<T::AccountId, Public, TrusteeAddrInfo> for Module<T> {
    fn check_trustee_entity(raw_addr: &[u8]) -> result::Result<Public, &'static str> {
        let public = Public::from_slice(raw_addr).map_err(|_| "Invalid Public")?;
        if let Public::Normal(_) = public {
            return Err("not allow Normal Public for bitcoin now");
        }
        Ok(public)
    }

    fn generate_trustee_session_info(
        props: Vec<(T::AccountId, TrusteeIntentionProps<Public>)>,
        config: TrusteeInfoConfig,
    ) -> result::Result<TrusteeSessionInfo<T::AccountId, TrusteeAddrInfo>, &'static str> {
        // judge all props has different pubkey
        // check
        let (trustees, props_info): (Vec<T::AccountId>, Vec<TrusteeIntentionProps<Public>>) =
            props.into_iter().unzip();

        let (hot_keys, cold_keys): (Vec<Public>, Vec<Public>) = props_info
            .into_iter()
            .map(|props| (props.hot_entity, props.cold_entity))
            .unzip();

        check_keys(&hot_keys)?;
        check_keys(&cold_keys)?;

        // [min, max] e.g. bitcoin min is 4, max is 15
        if (trustees.len() as u32) < config.min_trustee_count
            || (trustees.len() as u32) > config.max_trustee_count
        {
            error!("[generate_trustee_session_info]|trustees is less/more than {{min:[{:}], max:[{:}]}} people, can't generate trustee addr|trustees:{:?}",
                   config.min_trustee_count, config.max_trustee_count, trustees);
            return Err("trustees is less/more than required people, can't generate trustee addr");
        }
        info!(
            "[generate_trustee_session_info]|hot_keys:{:?}|cold_keys:{:?}",
            hot_keys, cold_keys
        );

        let sig_num = two_thirds_unsafe(trustees.len() as u32);

        let hot_trustee_addr_info: TrusteeAddrInfo = create_multi_address::<T>(&hot_keys, sig_num)
            .ok_or_else(|| {
                error!(
                    "[generate_trustee_session_info]|create hot_addr err!|hot_keys:{:?}",
                    hot_keys
                );
                "create hot_addr err!"
            })?;

        let cold_trustee_addr_info: TrusteeAddrInfo =
            create_multi_address::<T>(&cold_keys, sig_num).ok_or_else(|| {
                error!(
                    "[generate_trustee_session_info]|create cold_addr err!|cold_keys:{:?}",
                    cold_keys
                );
                "create cold_addr err!"
            })?;

        info!(
            "[generate_trustee_session_info]|hot_addr:{:?}|cold_addr:{:?}|trustee_list:{:?}",
            hot_trustee_addr_info,
            cold_trustee_addr_info,
            trustees!(trustees)
        );

        Ok(TrusteeSessionInfo {
            trustee_list: trustees,
            hot_address: hot_trustee_addr_info,
            cold_address: cold_trustee_addr_info,
        })
    }
}

impl<T: Trait> Module<T> {
    pub fn verify_btc_address(data: &[u8]) -> result::Result<BitcoinAddress, AddressError> {
        let r = b58::from(data).map_err(|_| AddressError::InvalidAddress)?;
        BitcoinAddress::from_layout(&r)
    }

    fn ensure_trustee(who: &T::AccountId) -> Result {
        let trustee_session_info = trustee_session::<T>()?;
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
            header.hash(),
        );
        // current should exist yet
        ensure_with_errorlog!(
            Self::block_header_for(&header.previous_header_hash).is_some(),
            "Can't find previous header",
            "prev hash:{:}|current hash:{:}",
            header.previous_header_hash,
            header.hash(),
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

        debug!("[apply_push_header]|verify pass, insert to storage|height:{:}|hash:{:}|block hashs for this height:{:?}",
            header_info.height,
            hash,
            Self::block_hash_for(header_info.height)
        );

        let best_header = match Self::block_header_for(Self::best_index()) {
            Some(info) => info,
            None => return Err("can't find the best header in ChainX"),
        };

        let (confirmed_hash, confirmed_height) = if header_info.height > best_header.height {
            header::remove_unused_headers::<T>(&header_info);

            let (confirmed_hash, confirmed_height) =
                header::update_confirmed_header::<T>(&header_info);
            info!(
                "[apply_push_header]|update to new height|height:{:}|hash:{:}",
                header_info.height, hash,
            );
            // change new best index
            BestIndex::<T>::put(hash);
            (confirmed_hash, confirmed_height)
        } else {
            info!("[apply_push_header]|best index larger than this height|best height:{:}|this height{:}",
                best_header.height,
                header_info.height
            );
            let info = header::find_confirmed_block::<T>(&hash);

            (info.header.hash(), info.height)
        };
        Self::deposit_event(RawEvent::InsertHeader(
            header_info.header.version,
            header_info.header.hash(),
            header_info.height,
            header_info.header.previous_header_hash,
            header_info.header.merkle_root_hash,
            header_info.header.time,
            header_info.header.nonce,
            confirmed_height,
            confirmed_hash,
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
            error!("[apply_push_transaction]|this block already has this tx|block_hash:{:}|tx_hash:{:}|tx_list:{:?}", tx.block_hash, tx_hash, header_info.txid_list);
            return Err("this block already has this tx");
        }

        // same tx may in different forked block, thus, just modify different forked block txlist, and the tx only insert once
        // so when the tx is existed, return tx_type, else set `TxFor` and `InputAddrFor` storage, return tx_type
        // get tx_type
        let (tx_type, _existed) = match Self::tx_for(&tx_hash) {
            None => {
                let (tx_type, input_addr) = detect_transaction_type::<T>(&tx)?;
                if tx_type == TxType::Irrelevance {
                    warn!("[apply_push_transaction]|this tx is not related to any important addr, maybe an irrelevance tx, drop it|relay_tx:{:?}|block_height:{:}", tx, height);
                    return Err("this tx is not related to any important addr, maybe an irrelevance tx, drop it");
                }
                // parse first input addr, may delete when only use opreturn to get accountid
                // only deposit tx store prev input tx's addr, for deposit to lookup related accountid
                if let Some(addr) = input_addr {
                    debug!(
                        "[apply_push_transaction]|deposit input addr|txhash:{:}|addr:{:}",
                        tx_hash,
                        u8array_to_string(&b58::to_base58(addr.layout().to_vec())),
                    );
                    InputAddrFor::<T>::insert(&tx_hash, addr)
                }
                // set tx into storage
                #[allow(deprecated)]
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
            tx_hash,
            _existed,
            tx_type,
            tx.block_hash,
            height,
            confirmed
        );

        // log event
        Self::deposit_event(RawEvent::InsertTx(
            tx_hash.clone(),
            tx.block_hash.clone(),
            tx_type,
        ));
        // if confirmed, handle this tx for deposit or withdrawal
        if confirmed {
            handle_tx::<T>(&tx_hash).map_err(|e| {
                error!("Handle tx error: {:}", tx_hash);
                e
            })?;
        }

        Ok(())
    }

    fn apply_create_withdraw(
        who: T::AccountId,
        tx: Transaction,
        withdrawal_id_list: Vec<u32>,
    ) -> Result {
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

        // check sig
        let sigs_count = parse_and_check_signed_tx::<T>(&tx)?;
        let apply_sig = if sigs_count == 0 {
            false
        } else if sigs_count == 1 {
            true
        } else {
            error!("[apply_create_withdraw]|the sigs for tx could not more than 1 in apply_create_withdraw|current sigs:{:}", sigs_count);
            return Err("the sigs for tx could not more than 1 in apply_create_withdraw");
        };

        xrecords::Module::<T>::withdrawal_processing(&withdrawal_id_list)?;
        // log event
        for id in withdrawal_id_list.iter() {
            Self::deposit_event(RawEvent::Withdrawal(*id, Vec::new(), TxState::Signing));
        }

        let mut proposal = WithdrawalProposal::new(
            VoteResult::Unfinish,
            withdrawal_id_list.clone(),
            tx,
            Vec::new(),
        );

        info!("[apply_create_withdraw]|Through the legality check of withdrawal");

        Self::deposit_event(RawEvent::CreateWithdrawalProposal(
            who.clone(),
            withdrawal_id_list,
        ));

        if apply_sig {
            info!("[apply_create_withdraw]apply sign after create proposal");
            // due to `SignWithdrawalProposal` event should after `CreateWithdrawalProposal`, thus this function should after proposal
            // but this function would have an error return, this error return should not meet.
            if let Err(s) = insert_trustee_vote_state::<T>(true, &who, &mut proposal.trustee_list) {
                // should not be error in this function, if hit this branch, panic to clear all modification
                panic!(s)
            }
        }

        CurrentWithdrawalProposal::<T>::put(proposal);

        Ok(())
    }

    fn apply_sig_withdraw(who: T::AccountId, tx: Option<Transaction>) -> Result {
        let mut proposal: WithdrawalProposal<T::AccountId> =
            Self::withdrawal_proposal().ok_or("No transactions waiting for signature")?;

        if proposal.sig_state == VoteResult::Finish {
            error!("[apply_sig_withdraw]|proposal is on FINISH state, can't sign for this proposal|proposal：{:?}", proposal);
            return Err("proposal is on FINISH state, can't sign for this proposal");
        }

        let (sig_num, total) = get_sig_num::<T>();
        match tx {
            Some(tx) => {
                // check this tx is same to proposal, just check input and output, not include sigs
                tx::utils::ensure_identical(&tx, &proposal.tx)?;

                // sign
                // check first and get signatures from commit transaction
                let sigs_count = parse_and_check_signed_tx::<T>(&tx)?;
                if sigs_count == 0 {
                    error!("[apply_sig_withdraw]|the tx sig should not be zero, zero is the source tx without any sig|tx{:?}", tx);
                    return Err("sigs count should not be zero for apply sig");
                }

                let confirmed_count = proposal
                    .trustee_list
                    .iter()
                    .filter(|(_, vote)| *vote == true)
                    .count() as u32;

                if sigs_count != confirmed_count + 1 {
                    error!(
                        "[apply_sig_withdraw]|Need to sign on the latest signature results|sigs count:{:}|confirmed count:{:}",
                        sigs_count,
                        confirmed_count
                    );
                    return Err("Need to sign on the latest signature results");
                }

                insert_trustee_vote_state::<T>(true, &who, &mut proposal.trustee_list)?;
                // check required count
                // required count should be equal or more than (2/3)*total
                // e.g. total=6 => required=2*6/3=4, thus equal to 4 should mark as finish
                if sigs_count == sig_num {
                    // mark as finish, can't do anything for this proposal
                    info!("[apply_sig_withdraw]Signature completed: {:}", sigs_count);
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
                insert_trustee_vote_state::<T>(false, &who, &mut proposal.trustee_list)?;

                let reject_count = proposal
                    .trustee_list
                    .iter()
                    .filter(|(_, vote)| *vote == false)
                    .count() as u32;

                // reject count just need  < (total-required) / total
                // e.g. total=6 => required=2*6/3=4, thus, reject should more than (6-4) = 2
                // > 2 equal to total - required + 1 = 6-4+1 = 3
                let need_reject = total - sig_num + 1;
                if reject_count == need_reject {
                    info!(
                        "[apply_sig_withdraw]|{:}/{:} opposition, clear withdrawal propoal",
                        reject_count, total
                    );

                    // release withdrawal for applications
                    for id in proposal.withdrawal_id_list.iter() {
                        let _ = xrecords::Module::<T>::withdrawal_recover_by_trustee(*id);
                    }

                    CurrentWithdrawalProposal::<T>::kill();

                    // log event
                    for id in proposal.withdrawal_id_list.iter() {
                        Self::deposit_event(RawEvent::Withdrawal(
                            *id,
                            Vec::new(),
                            TxState::Applying,
                        ));
                    }

                    Self::deposit_event(RawEvent::DropWithdrawalProposal(
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
