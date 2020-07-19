// Copyright 2018-2019 Chainpool.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

// mod assets_records;
pub mod header;
// pub mod lockup;
mod tests;
mod traits;
pub mod tx;
mod types;
pub mod trustee;

use codec::Decode;

// Substrate
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::{fmt::Debug, prelude::*, result};

use frame_support::{
    debug::native,
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
};
use frame_system::{self as system, ensure_signed};

// ChainX
use chainx_primitives::{AddrStr, AssetId, Memo};
use xpallet_assets::{Chain, ChainT};
// use xbridge_common::{
//     traits::{CrossChainBinding, Extractable, TrusteeForChain, TrusteeMultiSig, TrusteeSession},
//     types::{TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo},
//     utils::two_thirds_unsafe,
// };
// use xrecords::{WithdrawalState, TxState};
use xpallet_support::{
    base58, debug, ensure_with_errorlog, error, info, try_addr, warn, RUNTIME_TARGET,
};

// light-bitcoin
use btc_chain::Transaction;
use btc_keys::{Address as BTCAddress, DisplayLayout, Error as AddressError, Public};
use btc_ser::{deserialize, Reader};
// re-export
pub use btc_chain::BlockHeader as BTCHeader;
pub use btc_keys::Network as BTCNetwork;
#[cfg(feature = "std")]
pub use btc_primitives::h256_conv_endian_from_str;
pub use btc_primitives::{Compact, H256, H264};

use self::types::{
    BTCHeaderIndex, BTCHeaderInfo, BTCParams, BTCTxInfo, BTCTxType, RelayedTx, TrusteeAddrInfo,
    BTCTxVerifier, VoteResult, WithdrawalProposal, BTCTxState, BTCTxResult
};

pub trait Trait:
    frame_system::Trait
    + pallet_timestamp::Trait
    + xpallet_assets::Trait
    + xpallet_gateway_records::Trait // xsystem::Trait + xrecords::Trait + xbridge_common::Trait
{
    // type AccountExtractor: Extractable<Self::AccountId>;
    type TrusteeSessionProvider: TrusteeSession<Self::AccountId, TrusteeAddrInfo>;
    type TrusteeMultiSigProvider: TrusteeMultiSig<Self::AccountId>;
    // type CrossChainProvider: CrossChainBinding<Self::AccountId, BTCAddress>;
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_error! {
    /// Error for the XBridge Bitcoin module
    pub enum Error for Module<T: Trait> {
        /// parse base58 addr error
        InvalidBase58,
        /// load addr from bytes error
        InvalidAddr,
        /// can't find the best header in chain or it's invalid
        InvalidBestIndex,
        /// Invalid proof-of-work (Block hash does not satisfy nBits)
        InvalidPoW,
        /// Futuristic timestamp
        HeaderFuturisticTimestamp,
        /// nBits do not match difficulty rules
        HeaderNBitsNotMatch,
        /// Unknown parent
        HeaderUnknownParent,
        /// Not Found
        HeaderNotFound,
        /// Ancient fork
        HeaderAncientFork,
        /// Header already exists
        ExistedHeader,
        /// Can't find previous header
        PrevHeaderNotExisted,
        /// Cannot deserialize the header vec
        DeserializeHeaderErr,
        ///
        BadMerkleProof,
        /// reject unconfirmed transaction
        UnconfirmedTx,
        /// reject replay proccessed tx
        ReplayedTx,
        ///
        MismatchedTx,
        ///
        VerifySignFailed,
        ///
        InvalidSignCount,
        ///
        InvalidPublicKey,
        ///
        ConstructBadSign,
        /// Invalid signature
        BadSignature,
        /// Parse redeem script failed
        BadRedeemScript,

    }
}

decl_event!(
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId,
        <T as xpallet_assets::Trait>::Balance {
        /// block hash
        InsertHeader(H256),
        // /// tx hash, block hash, tx type
        ProcessTx(H256, H256, BTCTxState),
        // InsertTx(H256, H256, TxType),
        // /// who, Chain, AssetId, balance, memo, Chain Addr, chain txid, chain TxState
        // Deposit(AccountId, Chain, AssetId, Balance, Memo, AddrStr, Vec<u8>, TxState),
        // /// who, Chain, AssetId, balance,  Chain Addr
        // DepositPending(AccountId, Chain, AssetId, Balance, AddrStr),
        // /// who, withdrawal id, txid, TxState
        // Withdrawal(u32, Vec<u8>, TxState),
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
    trait Store for Module<T: Trait> as XGatewayBitcoin {
        /// best header info
        pub BestIndex get(fn best_index): BTCHeaderIndex;
        /// confirmed header info
        pub ConfirmedHeader get(fn confirmed_header): BTCHeaderIndex;
        /// block hash list for a height, include forked header hash
        pub BlockHashFor get(fn block_hash_for): map hasher(twox_64_concat) u32 => Vec<H256>;
        /// mark this blockhash is in mainchain
        pub MainChain get(fn main_chain): map hasher(identity) H256 => Option<()>;
        /// all valid blockheader (include forked blockheader)
        pub BTCHeaderFor get(fn btc_header_for): map hasher(identity) H256 => Option<BTCHeaderInfo>;

        /// mark tx has been handled, in case re-handle this tx, and log handle result
        pub TxState get(fn tx_state): map hash(identity) H256 => Option<BTCTxState>;

        // /// unclaim deposit info, addr => tx_hash, btc value, blockhash
        // pub PendingDepositMap get(fn pending_deposit): map hasher(blake2_128_concat) BTCAddress => Option<Vec<DepositCache>>;
        // /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
        // pub CurrentWithdrawalProposal get(fn withdrawal_proposal): Option<WithdrawalProposal<T::AccountId>>;

        /// get GenesisInfo (header, height)
        pub GenesisInfo get(fn genesis_info) config(): (BTCHeader, u32);
        /// get ParamsInfo from genesis_config
        pub ParamsInfo get(fn params_info) config(): BTCParams;
        ///  NetworkId for testnet or mainnet
        pub NetworkId get(fn network_id) config(): BTCNetwork;
        /// reserved count for block
        pub ReservedBlock get(fn reserved_block) config(): u32;
        /// get ConfirmationNumber from genesis_config
        pub ConfirmationNumber get(fn confirmation_number) config(): u32;
        /// get BTCWithdrawalFee from genesis_config
        pub BTCWithdrawalFee get(fn btc_withdrawal_fee) config(): u64;
        /// min deposit value limit, default is 10w sotashi(0.001 BTC)
        pub BTCMinDeposit get(fn btc_min_deposit): u64 = 1 * 100000;
        /// max withdraw account count in bitcoin withdrawal transaction
        pub MaxWithdrawalCount get(fn max_withdrawal_count) config(): u32;

        Verifier get(fn verifier) config(): BTCTxVerifier;
    }
    add_extra_genesis {
        config(genesis_hash): H256;
        build(|config| {
            let genesis_header = config.genesis_info.0.clone();
            let genesis_hash = genesis_header.hash();
            let genesis_index = BTCHeaderIndex {
                hash: genesis_hash,
                height: config.genesis_info.1
            };
            let header_info = BTCHeaderInfo {
                header: genesis_header,
                height: config.genesis_info.1
            };
            // would ignore check for bitcoin testnet
            #[cfg(not(test))] {
            if let BTCNetwork::Mainnet = config.network_id {
                if genesis_index.height % config.params_info.retargeting_interval() != 0 {
                    panic!("the blocknumber[{:}] should start from a changed difficulty block", genesis_index.height);
                }
            }
            }

            if genesis_hash != config.genesis_hash {
                panic!("the genesis block not much the genesis_hash!|genesis_block's hash:{:?}|config genesis_hash:{:?}", genesis_hash, config.genesis_hash);
            }


            BTCHeaderFor::insert(&genesis_hash, header_info);
            BlockHashFor::insert(&genesis_index.height, vec![genesis_hash.clone()]);
            MainChain::insert(&genesis_hash, ());

            BestIndex::put(genesis_index);
            ConfirmedHeader::put(genesis_index);

            Module::<T>::deposit_event(RawEvent::InsertHeader(
                genesis_hash,
            ));
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// if use `BTCHeader` struct would export in metadata, cause complex in front-end
        #[weight = 0]
        pub fn push_header(origin, header: Vec<u8>) -> DispatchResult {
            let _from = ensure_signed(origin)?;
            let header: BTCHeader = deserialize(header.as_slice()).map_err(|_| Error::<T>::DeserializeHeaderErr)?;
            debug!("[push_header]|from:{:?}|header:{:?}", _from, header);

            Self::apply_push_header(header)?;
            Ok(())
        }

        /// if use `RelayTx` struct would export in metadata, cause complex in front-end
        #[weight = 0]
        pub fn push_transaction(origin, tx: Vec<u8>) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let relay_tx: RelayTx = Decode::decode(&mut tx.as_slice()).ok_or("Parse RelayTx err")?;
            debug!("[push_transaction]|from:{:?}|relay_tx:{:?}", from, relay_tx);

            Self::apply_push_transaction(relay_tx)?;

            let post_info = PostDispatchInfo {
                actual_weight: Some(Zero::zero()),
            };
            Ok(post_info)
        }
        //
        // pub fn create_withdraw_tx(origin, withdrawal_id_list: Vec<u32>, tx: Vec<u8>) -> DispatchResult {
        //     let from = ensure_signed(origin)?;
        //     // commiter must in trustee list
        //     Self::ensure_trustee(&from)?;
        //
        //     let tx: Transaction = deserialize(Reader::new(tx.as_slice())).map_err(|_| "Parse transaction err")?;
        //     debug!("[create_withdraw_tx]|from:{:?}|withdrawal list:{:?}|tx:{:?}", from, withdrawal_id_list, tx);
        //
        //     Self::apply_create_withdraw(from, tx, withdrawal_id_list.clone())?;
        //     Ok(())
        // }
        //
        // pub fn sign_withdraw_tx(origin, tx: Option<Vec<u8>>) -> DispatchResult {
        //     let from = ensure_signed(origin)?;
        //     Self::ensure_trustee(&from)?;
        //
        //     let tx = if let Some(raw_tx) = tx {
        //         let tx: Transaction = deserialize(Reader::new(raw_tx.as_slice())).map_err(|_| "Parse transaction err")?;
        //         Some(tx)
        //     } else {
        //         None
        //     };
        //     debug!("[sign_withdraw_tx]|from:{:?}|vote_tx:{:?}", from, tx);
        //
        //     Self::apply_sig_withdraw(from, tx)?;
        //     Ok(())
        // }
        //
        // pub fn fix_withdrawal_state_by_trustees(origin, withdrawal_id: u32, state: WithdrawalState) -> DispatchResult {
        //     let from = ensure_signed(origin)?;
        //     T::TrusteeMultiSigProvider::check_multisig(&from)?;
        //     xrecords::Module::<T>::fix_withdrawal_state_by_trustees(Chain::Bitcoin, withdrawal_id, state)
        // }
        //
        // pub fn set_btc_withdrawal_fee_by_trustees(origin, fee: T::Balance) -> DispatchResult {
        //     let from = ensure_signed(origin)?;
        //     T::TrusteeMultiSigProvider::check_multisig(&from)?;
        //
        //     Self::set_btc_withdrawal_fee(fee)
        // }
        //
        // pub fn remove_tx_and_proposal(txhash: Option<H256>, drop_proposal: bool) -> DispatchResult {
        //     if let Some(hash) = txhash {
        //         TxFor::<T>::remove(&hash);
        //         InputAddrFor::<T>::remove(&hash);
        //     }
        //     if drop_proposal {
        //         CurrentWithdrawalProposal::<T>::kill();
        //     }
        //     Ok(())
        // }
        //
        // pub fn set_btc_withdrawal_fee(fee: T::Balance) -> DispatchResult {
        //     BTCWithdrawalFee::<T>::put(fee.into());
        //     Ok(())
        // }
        //
        // pub fn set_btc_deposit_limit(value: T::Balance) {
        //     BTCMinDeposit::<T>::put(value.into());
        // }
        //
        // pub fn set_btc_deposit_limit_by_trustees(origin, value: T::Balance) {
        //     let from = ensure_signed(origin)?;
        //     T::TrusteeMultiSigProvider::check_multisig(&from)?;
        //
        //     let _ = Self::set_btc_deposit_limit(value);
        // }
        //
        // pub fn remove_pending(addr: BTCAddress, who: Option<T::AccountId>) -> DispatchResult {
        //     if let Some(w) = who {
        //         remove_pending_deposit::<T>(&addr, &w);
        //     } else {
        //         info!("[remove_pending]|release pending deposit directly, not deposit to someone|addr:{:?}", addr);
        //         PendingDepositMap::<T>::remove(&addr);
        //     }
        //     Ok(())
        // }
        //
        // pub fn remove_pending_by_trustees(origin, addr: BTCAddress, who: Option<T::AccountId>) -> DispatchResult {
        //     let from = ensure_signed(origin)?;
        //     T::TrusteeMultiSigProvider::check_multisig(&from)?;
        //     Self::remove_pending(addr, who)
        // }
        //
        // /// Dangerous! Be careful to set BestIndex
        // pub fn set_best_index(hash: H256) {
        //     warn!("[set_best_index]|Dangerous! set new best index|hash:{:?}", hash);
        //     BestIndex::<T>::put(hash);
        // }
        //
        // pub fn set_header_confirmed_state(hash: H256, confirmed: bool) {
        //     BTCHeaderFor::mutate(hash, |info| {
        //         if let Some(info) = info {
        //             warn!("[set_header_confirmed_state]|modify header confirmed state|hash:{:?}|confirmed:{:}", hash, confirmed);
        //             info.confirmed = confirmed;
        //         }
        //     })
        // }
    }
}

impl<T: Trait> ChainT for Module<T> {
    const ASSET_ID: AssetId = xpallet_protocol::X_BTC;

    fn chain() -> Chain {
        Chain::Bitcoin
    }

    fn check_addr(addr: &[u8], _: &[u8]) -> DispatchResult {
        // this addr is base58 addr
        let address = Self::verify_btc_address(addr)
            .map_err(|_| "Verify btc addr err")
            .map_err(|e| {
                error!(
                    "[verify_btc_address]|failed, source addr is:{:?}",
                    try_addr!(addr)
                );
                e
            })?;

        // let (hot_addr, cold_addr) = get_trustee_address_pair::<T>()?;
        // if address == hot_addr || address == cold_addr {
        //     return Err("current addr is equal to hot or cold trustee addr");
        // }

        Ok(())
    }
}

// fn check_keys(keys: &[Public]) -> DispatchResult {
//     let has_duplicate = (1..keys.len()).any(|i| keys[i..].contains(&keys[i - 1]));
//     if has_duplicate {
//         error!("[generate_new_trustees]|keys contains duplicate pubkey");
//         return Err("keys contains duplicate pubkey");
//     }
//     if keys.iter().any(|public: &Public| {
//         if let Public::Normal(_) = public {
//             true
//         } else {
//             false
//         }
//     }) {
//         return Err("unexpect! all keys(bitcoin Public) should be compressed");
//     }
//     Ok(())
// }

//const EC_P = Buffer.from('fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f', 'hex')
const EC_P: [u8; 32] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255, 252, 47,
];

const ZERO_P: [u8; 32] = [0; 32];
/*
impl<T: Trait> TrusteeForChain<T::AccountId, Public, TrusteeAddrInfo> for Module<T> {
    fn check_trustee_entity(raw_addr: &[u8]) -> result::DispatchResult<Public, &'static str> {
        let public = Public::from_slice(raw_addr).map_err(|_| "Invalid Public")?;
        if let Public::Normal(_) = public {
            return Err("not allow Normal Public for bitcoin now");
        }

        if 2 != raw_addr[0] && 3 != raw_addr[0] {
            return Err("not Compressed Public(prefix not 2|3)");
        }

        if &ZERO_P == &raw_addr[1..33] {
            return Err("not Compressed Public(Zero32)");
        }

        if &raw_addr[1..33] >= &EC_P {
            return Err("not Compressed Public(EC_P)");
        }

        Ok(public)
    }

    fn generate_trustee_session_info(
        props: Vec<(T::AccountId, TrusteeIntentionProps<Public>)>,
        config: TrusteeInfoConfig,
    ) -> result::DispatchResult<TrusteeSessionInfo<T::AccountId, TrusteeAddrInfo>, &'static str> {
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
*/
impl<T: Trait> Module<T> {
    pub fn verify_btc_address(data: &[u8]) -> result::Result<BTCAddress, DispatchError> {
        let r = base58::from(data).map_err(|_| Error::<T>::InvalidBase58)?;
        let addr = BTCAddress::from_layout(&r).map_err(|_| Error::<T>::InvalidAddr)?;
        Ok(addr)
    }

    // fn ensure_trustee(who: &T::AccountId) -> DispatchResult {
    //     let trustee_session_info = trustee_session::<T>()?;
    //     if trustee_session_info.trustee_list.iter().any(|n| n == who) {
    //         return Ok(());
    //     }
    //     error!(
    //         "[ensure_trustee]|Committer not in the trustee list!|who:{:?}|trustees:{:?}",
    //         who, trustee_session_info.trustee_list
    //     );
    //     Err("Committer not in the trustee list")
    // }

    fn apply_push_header(header: BTCHeader) -> DispatchResult {
        // current should not exist
        ensure_with_errorlog!(
            Self::btc_header_for(&header.hash()).is_none(),
            Error::<T>::ExistedHeader,
            "Header already exists|hash:{:}",
            header.hash(),
        );
        // prev header should exist, thus we reject orphan block
        let prev_info = Self::btc_header_for(header.previous_header_hash).ok_or_else(|| {
            native::error!(
                target: RUNTIME_TARGET,
                "[check_prev_and_convert]|not find prev header|current header:{:?}",
                header
            );
            Error::<T>::PrevHeaderNotExisted
        })?;

        // convert btc header to self header info
        let header_info: BTCHeaderInfo = BTCHeaderInfo {
            header,
            height: prev_info.height + 1,
        };
        // check
        let c =
            header::HeaderVerifier::new::<T>(&header_info).map_err::<Error<T>, _>(Into::into)?;
        c.check::<T>()?;

        // insert into storage
        let hash = header_info.header.hash();
        // insert valid header into storage
        BTCHeaderFor::insert(&hash, header_info.clone());
        // storage height => block list (contains forked header hash)
        BlockHashFor::mutate(header_info.height, |v| {
            if !v.contains(&hash) {
                v.push(hash.clone());
            }
        });

        debug!("[apply_push_header]|verify pass, insert to storage|height:{:}|hash:{:?}|block hashs for this height:{:?}",
            header_info.height,
            hash,
            Self::block_hash_for(header_info.height)
        );

        let best_index = Self::best_index();

        if header_info.height > best_index.height {
            // new best index
            let new_best_index = BTCHeaderIndex {
                hash,
                height: header_info.height,
            };

            let confirmed_index = header::update_confirmed_header::<T>(&header_info);
            info!(
                "[apply_push_header]|update to new height|height:{:}|hash:{:?}",
                header_info.height, hash,
            );
            // change new best index
            BestIndex::put(new_best_index);
        } else {
            info!("[apply_push_header]|best index larger than this height|best height:{:}|this height{:}", best_index.height, header_info.height);
            // let info = header::find_confirmed_block::<T>(&hash);
            // (info.header.hash(), info.height)
        };
        Self::deposit_event(RawEvent::InsertHeader(hash));

        Ok(())
    }

    fn apply_push_transaction(tx: RelayedTx) -> DispatchResult {
        let tx_hash = tx.raw.hash();
        let block_hash = tx.block_hash;
        let mut header_info = Module::<T>::btc_header_for(&tx.block_hash).ok_or_else(|| {
            error!(
                "[apply_push_transaction]|tx's block header must exist before|block_hash:{:}",
                block_hash
            );
            "tx's block header must exist before"
        })?;
        let merkle_root = header_info.header.merkle_root_hash;
        // verify, check merkle proof
        tx::validate_transaction::<T>(&tx, merkle_root)?;

        let confirmed = Self::confirmed_header();
        let height = header_info.height;
        if height > confirmed.height {
            error!("[apply_push_transaction]|receive an unconfirmed tx|tx hash:{:}|related block height:{:}|confirmed block height:{:}|hash:{:?}", tx_hash, height, confirmed.height, confirmed.hash);
            Err(Error::<T>::UnconfirmedTx)?;
        }
        // protect replayed tx, just process failed and not processed tx;
        match Self::tx_state(&tx_hash) {
            None => { /* do nothing */ },
            Some(state) => {
                if state == BTCTxResult::Success {
                    error!("[apply_push_transaction]|reject processed tx|tx hash:{:}|type:{:?}|result:{:?}", tx_hash, state.tx_type, state.result);
                    Err(Error::<T>::ReplayedTx)?;
                }
            }
        }

        let state = tx::process_tx::<T>(tx.raw)?;
        // set storage
        TxState::insert(&tx_hash, state);
        Self::deposit_event(RawEvent::ProcessTx(tx_hash, block_hash, state));
        Ok(())
    }

    // fn apply_create_withdraw(
    //     who: T::AccountId,
    //     tx: Transaction,
    //     withdrawal_id_list: Vec<u32>,
    // ) -> DispatchResult {
    //     let withdraw_amount = Self::max_withdrawal_count();
    //     if withdrawal_id_list.len() > withdraw_amount as usize {
    //         error!("[apply_create_withdraw]|Exceeding the maximum withdrawal amount|current list len:{:}|max:{:}", withdrawal_id_list.len(), withdraw_amount);
    //         return Err("Exceeding the maximum withdrawal amount");
    //     }
    //     // remove duplicate
    //     let mut withdrawal_id_list = withdrawal_id_list;
    //     withdrawal_id_list.sort();
    //     withdrawal_id_list.dedup();
    //
    //     check_withdraw_tx::<T>(&tx, &withdrawal_id_list)?;
    //     info!(
    //         "[apply_create_withdraw]|create new withdraw|withdrawal idlist:{:?}",
    //         withdrawal_id_list
    //     );
    //
    //     // check sig
    //     let sigs_count = parse_and_check_signed_tx::<T>(&tx)?;
    //     let apply_sig = if sigs_count == 0 {
    //         false
    //     } else if sigs_count == 1 {
    //         true
    //     } else {
    //         error!("[apply_create_withdraw]|the sigs for tx could not more than 1 in apply_create_withdraw|current sigs:{:}", sigs_count);
    //         return Err("the sigs for tx could not more than 1 in apply_create_withdraw");
    //     };
    //
    //     xrecords::Module::<T>::withdrawal_processing(&withdrawal_id_list)?;
    //     // log event
    //     for id in withdrawal_id_list.iter() {
    //         Self::deposit_event(RawEvent::Withdrawal(*id, Vec::new(), TxState::Signing));
    //     }
    //
    //     let mut proposal = WithdrawalProposal::new(
    //         VoteResult::Unfinish,
    //         withdrawal_id_list.clone(),
    //         tx,
    //         Vec::new(),
    //     );
    //
    //     info!("[apply_create_withdraw]|Through the legality check of withdrawal");
    //
    //     Self::deposit_event(RawEvent::CreateWithdrawalProposal(
    //         who.clone(),
    //         withdrawal_id_list,
    //     ));
    //
    //     if apply_sig {
    //         info!("[apply_create_withdraw]apply sign after create proposal");
    //         // due to `SignWithdrawalProposal` event should after `CreateWithdrawalProposal`, thus this function should after proposal
    //         // but this function would have an error return, this error return should not meet.
    //         if let Err(s) = insert_trustee_vote_state::<T>(true, &who, &mut proposal.trustee_list) {
    //             // should not be error in this function, if hit this branch, panic to clear all modification
    //             panic!(s)
    //         }
    //     }
    //
    //     CurrentWithdrawalProposal::<T>::put(proposal);
    //
    //     Ok(())
    // }
    //
    // fn apply_sig_withdraw(who: T::AccountId, tx: Option<Transaction>) -> DispatchResult {
    //     let mut proposal: WithdrawalProposal<T::AccountId> =
    //         Self::withdrawal_proposal().ok_or("No transactions waiting for signature")?;
    //
    //     if proposal.sig_state == VoteResult::Finish {
    //         error!("[apply_sig_withdraw]|proposal is on FINISH state, can't sign for this proposal|proposal：{:?}", proposal);
    //         return Err("proposal is on FINISH state, can't sign for this proposal");
    //     }
    //
    //     let (sig_num, total) = get_sig_num::<T>();
    //     match tx {
    //         Some(tx) => {
    //             // check this tx is same to proposal, just check input and output, not include sigs
    //             tx::utils::ensure_identical(&tx, &proposal.tx)?;
    //
    //             // sign
    //             // check first and get signatures from commit transaction
    //             let sigs_count = parse_and_check_signed_tx::<T>(&tx)?;
    //             if sigs_count == 0 {
    //                 error!("[apply_sig_withdraw]|the tx sig should not be zero, zero is the source tx without any sig|tx{:?}", tx);
    //                 return Err("sigs count should not be zero for apply sig");
    //             }
    //
    //             let confirmed_count = proposal
    //                 .trustee_list
    //                 .iter()
    //                 .filter(|(_, vote)| *vote == true)
    //                 .count() as u32;
    //
    //             if sigs_count != confirmed_count + 1 {
    //                 error!(
    //                     "[apply_sig_withdraw]|Need to sign on the latest signature results|sigs count:{:}|confirmed count:{:}",
    //                     sigs_count,
    //                     confirmed_count
    //                 );
    //                 return Err("Need to sign on the latest signature results");
    //             }
    //
    //             insert_trustee_vote_state::<T>(true, &who, &mut proposal.trustee_list)?;
    //             // check required count
    //             // required count should be equal or more than (2/3)*total
    //             // e.g. total=6 => required=2*6/3=4, thus equal to 4 should mark as finish
    //             if sigs_count == sig_num {
    //                 // mark as finish, can't do anything for this proposal
    //                 info!("[apply_sig_withdraw]Signature completed: {:}", sigs_count);
    //                 proposal.sig_state = VoteResult::Finish;
    //
    //                 // log event
    //                 for id in proposal.withdrawal_id_list.iter() {
    //                     Self::deposit_event(RawEvent::Withdrawal(
    //                         *id,
    //                         Vec::new(),
    //                         TxState::Broadcasting,
    //                     ));
    //                 }
    //             } else {
    //                 proposal.sig_state = VoteResult::Unfinish;
    //             }
    //             // update tx
    //             proposal.tx = tx;
    //         }
    //         None => {
    //             // reject
    //             insert_trustee_vote_state::<T>(false, &who, &mut proposal.trustee_list)?;
    //
    //             let reject_count = proposal
    //                 .trustee_list
    //                 .iter()
    //                 .filter(|(_, vote)| *vote == false)
    //                 .count() as u32;
    //
    //             // reject count just need  < (total-required) / total
    //             // e.g. total=6 => required=2*6/3=4, thus, reject should more than (6-4) = 2
    //             // > 2 equal to total - required + 1 = 6-4+1 = 3
    //             let need_reject = total - sig_num + 1;
    //             if reject_count == need_reject {
    //                 info!(
    //                     "[apply_sig_withdraw]|{:}/{:} opposition, clear withdrawal propoal",
    //                     reject_count, total
    //                 );
    //
    //                 // release withdrawal for applications
    //                 for id in proposal.withdrawal_id_list.iter() {
    //                     let _ = xrecords::Module::<T>::withdrawal_recover_by_trustee(*id);
    //                 }
    //
    //                 CurrentWithdrawalProposal::<T>::kill();
    //
    //                 // log event
    //                 for id in proposal.withdrawal_id_list.iter() {
    //                     Self::deposit_event(RawEvent::Withdrawal(
    //                         *id,
    //                         Vec::new(),
    //                         TxState::Applying,
    //                     ));
    //                 }
    //
    //                 Self::deposit_event(RawEvent::DropWithdrawalProposal(
    //                     reject_count as u32,
    //                     sig_num as u32,
    //                     proposal.withdrawal_id_list.clone(),
    //                 ));
    //                 return Ok(());
    //             }
    //         }
    //     }
    //
    //     info!(
    //         "[apply_sig_withdraw]|current sig|state:{:?}|trustee vote:{:?}",
    //         proposal.sig_state, proposal.trustee_list
    //     );
    //
    //     CurrentWithdrawalProposal::<T>::put(proposal);
    //     Ok(())
    // }
}
