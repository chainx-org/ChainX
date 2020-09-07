// Copyright 2018-2019 Chainpool.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
pub mod header;
mod tests;
pub mod trustee;
pub mod tx;
mod types;
mod weight_info;

// Substrate
use sp_runtime::{traits::Zero, SaturatedConversion};
use sp_std::{prelude::*, result};

use frame_support::{
    debug::native,
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
    ensure,
    traits::{Currency, EnsureOrigin, UnixTime},
    weights::Pays,
};
use frame_system::{ensure_root, ensure_signed};

use orml_utilities::with_transaction_result;

// ChainX
use chainx_primitives::{AddrStr, AssetId};
use xpallet_assets::{Chain, ChainT, WithdrawalLimit};
use xpallet_gateway_common::{
    traits::{AddrBinding, ChannelBinding, Extractable, TrusteeSession},
    trustees::bitcoin::BtcTrusteeAddrInfo,
};
use xpallet_support::{debug, ensure_with_errorlog, error, info, str, try_addr};

// light-bitcoin
#[cfg(feature = "std")]
pub use light_bitcoin::primitives::h256_conv_endian_from_str;
pub use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    keys::Network as BtcNetwork,
    primitives::{Compact, H256, H264},
};
use light_bitcoin::{
    chain::Transaction,
    keys::{Address, DisplayLayout},
    serialization::{deserialize, Reader},
};

pub use self::types::{BtcAddress, BtcParams, BtcTxVerifier, BtcWithdrawalProposal};
use self::types::{
    BtcDepositCache, BtcHeaderIndex, BtcHeaderInfo, BtcRelayedTx, BtcRelayedTxInfo, BtcTxResult,
    BtcTxState,
};
use crate::trustee::get_trustee_address_pair;
use crate::tx::remove_pending_deposit;
use crate::weight_info::WeightInfo;

pub type BalanceOf<T> = <<T as xpallet_assets::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait:
    frame_system::Trait + xpallet_assets::Trait + xpallet_gateway_records::Trait
{
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type UnixTime: UnixTime;
    type AccountExtractor: Extractable<Self::AccountId>;
    type TrusteeSessionProvider: TrusteeSession<Self::AccountId, BtcTrusteeAddrInfo>;
    type TrusteeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;
    type Channel: ChannelBinding<Self::AccountId>;
    type AddrBinding: AddrBinding<Self::AccountId, BtcAddress>;
    type WeightInfo: WeightInfo;
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
        /// Fork is too long to proceed
        AncientFork,
        /// Previous tx id not equal input point hash
        InvalidPrevTx,
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
        /// Cannot deserialize the header or tx vec
        DeserializeErr,
        ///
        BadMerkleProof,
        /// reject unconfirmed transaction
        UnconfirmedTx,
        /// reject replay proccessed tx
        ReplayedTx,
        /// process tx failed
        ProcessTxFailed,
        /// withdraw tx not match expected tx
        MismatchedTx,
        /// invalid bitcoin address
        InvalidAddress,
        /// verify tx signature failed
        VerifySignFailed,
        /// invalid sign count in trustee withdrawal tx proposal
        InvalidSignCount,
        /// invalid bitcoin public key
        InvalidPublicKey,
        /// construct bad signature
        ConstructBadSign,
        /// Invalid signature
        BadSignature,
        /// Parse redeem script failed
        BadRedeemScript,
        /// not set trustee yet
        NotTrustee,
        /// duplicated pubkey for trustees
        DuplicatedKeys,
        /// can't generate multisig address
        GenerateMultisigFailed,
        /// invalid trustee count
        InvalidTrusteeCount,
        /// unexpected withdraw records count
        WroungWithdrawalCount,
        /// reject sig for current proposal
        RejectSig,
        /// no proposal for current withdrawal
        NoProposal,
        /// invalid proposal
        InvalidProposal,
        /// last proposal not finished yet
        NotFinishProposal,
        /// no withdrawal record for this id
        NoWithdrawalRecord,
    }
}

decl_event!(
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>
        {
        /// block hash
        InsertHeader(H256),
        /// tx hash, block hash, tx type
        ProcessTx(H256, H256, BtcTxState),
        /// who, balance, txhsah, Chain Addr
        DepositPending(AccountId, Balance, H256, AddrStr),
        /// create withdraw tx, who proposal, withdrawal list id
        CreateWithdrawalProposal(AccountId, Vec<u32>),
        /// Sign withdraw tx
        SignWithdrawalProposal(AccountId, bool),
        /// finish proposal and wait for broadcasting
        FinishProposal(H256),
        /// WithdrawalFatalErr, tx hash, Proposal hash,
        WithdrawalFatalErr(H256, H256),
        /// reject_count, sum_count, withdrawal id list
        DropWithdrawalProposal(u32, u32, Vec<u32>),
        /// Deposit token for a account.
        DepositToken(H256, AccountId, Balance),
        /// Withdraw token for a list of withdrawal applications.
        WithdrawToken(H256, Vec<u32>, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XGatewayBitcoin {
        /// best header info
        pub BestIndex get(fn best_index): BtcHeaderIndex;
        /// confirmed header info
        pub ConfirmedIndex get(fn confirmed_index): Option<BtcHeaderIndex>;
        /// block hash list for a height, include forked header hash
        pub BlockHashFor get(fn block_hash_for): map hasher(twox_64_concat) u32 => Vec<H256>;
        /// mark this blockhash is in mainchain
        pub MainChain get(fn main_chain): map hasher(identity) H256 => Option<()>;
        /// all valid blockheader (include forked blockheader)
        pub Headers get(fn headers): map hasher(identity) H256 => Option<BtcHeaderInfo>;

        /// mark tx has been handled, in case re-handle this tx, and log handle result
        pub TxState get(fn tx_state): map hasher(identity) H256 => Option<BtcTxState>;
        /// unclaimed deposit info, addr => tx_hash, btc value,
        pub PendingDeposits get(fn pending_deposits): map hasher(blake2_128_concat) BtcAddress => Vec<BtcDepositCache>;

        /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
        pub WithdrawalProposal get(fn withdrawal_proposal): Option<BtcWithdrawalProposal<T::AccountId>>;

        /// get GenesisInfo (header, height)
        pub GenesisInfo get(fn genesis_info) config(): (BtcHeader, u32);
        /// get ParamsInfo from genesis_config
        pub ParamsInfo get(fn params_info) config(): BtcParams;
        ///  NetworkId for testnet or mainnet
        pub NetworkId get(fn network_id) config(): BtcNetwork;
        /// reserved count for block
        pub ReservedBlock get(fn reserved_block) config(): u32;
        /// get ConfirmationNumber from genesis_config
        pub ConfirmationNumber get(fn confirmation_number) config(): u32;
        /// get BtcWithdrawalFee from genesis_config
        pub BtcWithdrawalFee get(fn btc_withdrawal_fee) config(): u64;
        /// min deposit value limit, default is 10w sotashi(0.001 BTC)
        pub BtcMinDeposit get(fn btc_min_deposit): u64 = 1 * 100000;
        /// max withdraw account count in bitcoin withdrawal transaction
        pub MaxWithdrawalCount get(fn max_withdrawal_count) config(): u32;

        Verifier get(fn verifier) config(): BtcTxVerifier;
    }
    add_extra_genesis {
        config(genesis_hash): H256;
        build(|config| {
            let genesis_header = config.genesis_info.0.clone();
            let genesis_hash = genesis_header.hash();
            let genesis_index = BtcHeaderIndex {
                hash: genesis_hash,
                height: config.genesis_info.1
            };
            let header_info = BtcHeaderInfo {
                header: genesis_header,
                height: config.genesis_info.1
            };
            // would ignore check for bitcoin testnet
            #[cfg(not(test))] {
            if let BtcNetwork::Mainnet = config.network_id {
                if genesis_index.height % config.params_info.retargeting_interval() != 0 {
                    panic!("the blocknumber[{:}] should start from a changed difficulty block", genesis_index.height);
                }
            }
            }

            if genesis_hash != config.genesis_hash {
                panic!("the genesis block not much the genesis_hash!|genesis_block's hash:{:?}|config genesis_hash:{:?}", genesis_hash, config.genesis_hash);
            }

            Headers::insert(&genesis_hash, header_info);
            BlockHashFor::insert(&genesis_index.height, vec![genesis_hash.clone()]);
            MainChain::insert(&genesis_hash, ());

            BestIndex::put(genesis_index);
            // ConfirmedIndex::put(genesis_index);

            Module::<T>::deposit_event(RawEvent::InsertHeader(
                genesis_hash,
            ));
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        /// if use `BtcHeader` struct would export in metadata, cause complex in front-end
        #[weight = <T as Trait>::WeightInfo::push_header()]
        pub fn push_header(origin, header: Vec<u8>) -> DispatchResultWithPostInfo {
            let _from = ensure_signed(origin)?;
            let header: BtcHeader = deserialize(header.as_slice()).map_err(|_| Error::<T>::DeserializeErr)?;
            debug!("[push_header]|from:{:?}|header:{:?}", _from, header);

            Self::apply_push_header(header)?;

            let post_info = PostDispatchInfo {
                actual_weight: Some(Zero::zero()),
                pays_fee: Pays::No,
            };
            Ok(post_info)
        }

        /// if use `RelayTx` struct would export in metadata, cause complex in front-end
        #[weight = <T as Trait>::WeightInfo::push_transaction()]
        pub fn push_transaction(origin, raw_tx: Vec<u8>, relayed_info: BtcRelayedTxInfo, prev_tx: Option<Vec<u8>>) -> DispatchResultWithPostInfo {
            let _from = ensure_signed(origin)?;
            let raw_tx: Transaction = deserialize(Reader::new(raw_tx.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
            let prev = if let Some(prev) = prev_tx {
                let prev: Transaction = deserialize(Reader::new(prev.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
                Some(prev)
            } else {
                None
            };
            let relay_tx = relayed_info.into_relayed_tx(raw_tx);
            native::debug!(
                target: xpallet_support::RUNTIME_TARGET,
                "[push_transaction]|from:{:?}|relay_tx:{:?}|prev:{:?}", _from, relay_tx, prev
            );

            Self::apply_push_transaction(relay_tx, prev)?;

            let post_info = PostDispatchInfo {
                actual_weight: Some(Zero::zero()),
                pays_fee: Pays::No,
            };
            Ok(post_info)
        }

        /// Trustee create a proposal for a withdrawal list. `tx` is the proposal withdrawal transaction.
        /// The `tx` would have a sign for current creator or do not have sign. if creator do not sign
        /// for this transaction, he could do `sign_withdraw_tx` later.
        #[weight = <T as Trait>::WeightInfo::create_withdraw_tx()]
        pub fn create_withdraw_tx(origin, withdrawal_id_list: Vec<u32>, tx: Vec<u8>) -> DispatchResult {
            let from = ensure_signed(origin)?;
            // commiter must in trustee list
            Self::ensure_trustee(&from)?;

            let tx: Transaction = deserialize(Reader::new(tx.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
            native::debug!(target: xpallet_support::RUNTIME_TARGET, "[create_withdraw_tx]|from:{:?}|withdrawal list:{:?}|tx:{:?}", from, withdrawal_id_list, tx);

            Self::apply_create_withdraw(from, tx, withdrawal_id_list.clone())?;
            Ok(())
        }

        /// Trustees sign a withdrawal proposal. If `tx` is None, means this trustee vote to reject
        /// this proposal. If `tx` is Some(), the inner part must be a valid transaction with this
        /// trustee signature.
        #[weight = <T as Trait>::WeightInfo::sign_withdraw_tx()]
        pub fn sign_withdraw_tx(origin, tx: Option<Vec<u8>>) -> DispatchResult {
            let from = ensure_signed(origin)?;
            Self::ensure_trustee(&from)?;

            let tx = if let Some(raw_tx) = tx {
                let tx: Transaction = deserialize(Reader::new(raw_tx.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
                Some(tx)
            } else {
                None
            };
            native::debug!(target: xpallet_support::RUNTIME_TARGET, "[sign_withdraw_tx]|from:{:?}|vote_tx:{:?}", from, tx);

            Self::apply_sig_withdraw(from, tx)?;
            Ok(())
        }

        /// Dangerous! Be careful to set BestIndex
        #[weight = <T as Trait>::WeightInfo::set_best_index()]
        pub fn set_best_index(origin, index: BtcHeaderIndex) -> DispatchResult {
            ensure_root(origin)?;
            BestIndex::put(index);
            Ok(())
        }
        /// Dangerous! Be careful to set ConfirmedIndex
        #[weight = <T as Trait>::WeightInfo::set_confirmed_index()]
        pub fn set_confirmed_index(origin, index: BtcHeaderIndex) -> DispatchResult {
            ensure_root(origin)?;
            ConfirmedIndex::put(index);
            Ok(())
        }

        /// Allow root or trustees could remove pending deposits for an address and decide whether
        /// deposit to an account id. if pass `None` to `who`, would just remove pendings, if pass
        /// Some, would deposit to this account id.
        #[weight = <T as Trait>::WeightInfo::remove_pending()]
        pub fn remove_pending(origin, addr: BtcAddress, who: Option<T::AccountId>) -> DispatchResult {
            T::TrusteeOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

            if let Some(w) = who {
                remove_pending_deposit::<T>(&addr, &w);
            } else {
                info!("[remove_pending]|release pending deposit directly, not deposit to someone|addr:{:?}", str!(&addr));
                PendingDeposits::remove(&addr);
            }
            Ok(())
        }

        /// Dangerous! remove current withdrawal proposal directly. Please check business logic before
        /// do this operation.
        #[weight = <T as Trait>::WeightInfo::remove_proposal()]
        pub fn remove_proposal(origin) -> DispatchResult {
            ensure_root(origin)?;
            WithdrawalProposal::<T>::kill();
            Ok(())
        }

        /// Dangerous! force replace current withdrawal proposal transaction. Please check business
        /// logic before do this operation. Must make sure current proposal transaction is invalid
        /// (e.g. when created a proposal, the inputs are not in double spend state, but after other
        /// trustees finish signing, the inputs are in double spend due other case. Thus could create
        /// a new valid transaction which outputs same to current proposal to replace current proposal
        /// transaction.)
        #[weight = <T as Trait>::WeightInfo::force_replace_proposal_tx()]
        pub fn force_replace_proposal_tx(origin, tx: Vec<u8>) -> DispatchResult {
            T::TrusteeOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;
            let tx: Transaction = deserialize(Reader::new(tx.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
            native::debug!(
                target: xpallet_support::RUNTIME_TARGET,
                "[force_replace_proposal_tx]|new_tx:{:?}", tx,
            );
            Self::force_replace_withdraw_tx(tx)
        }

        /// Set bitcoin withdrawal fee
        #[weight = <T as Trait>::WeightInfo::set_btc_withdrawal_fee()]
        pub fn set_btc_withdrawal_fee(origin, #[compact] fee: u64) -> DispatchResult {
            T::TrusteeOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;
            BtcWithdrawalFee::put(fee);
            Ok(())
        }

        /// Set bitcoin deposit limit
        #[weight = <T as Trait>::WeightInfo::set_btc_deposit_limit()]
        pub fn set_btc_deposit_limit(origin, #[compact] value: u64) -> DispatchResult {
            T::TrusteeOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;
            BtcMinDeposit::put(value);
            Ok(())
        }
    }
}

impl<T: Trait> ChainT<BalanceOf<T>> for Module<T> {
    const ASSET_ID: AssetId = xpallet_protocol::X_BTC;

    fn chain() -> Chain {
        Chain::Bitcoin
    }

    fn check_addr(addr: &[u8], _: &[u8]) -> DispatchResult {
        // this addr is base58 addr
        let address = Self::verify_btc_address(addr).map_err(|e| {
            error!(
                "[verify_btc_address]|failed, source addr is:{:?}",
                try_addr!(addr)
            );
            e
        })?;

        match get_trustee_address_pair::<T>() {
            Ok((hot_addr, cold_addr)) => {
                // do not allow withdraw from trustee address
                if address == hot_addr || address == cold_addr {
                    Err(Error::<T>::InvalidAddress)?;
                }
            }
            Err(e) => {
                error!("[check_addr]|not get trustee addr|err:{:?}", e);
            }
        }

        Ok(())
    }

    fn withdrawal_limit(
        asset_id: &AssetId,
    ) -> result::Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
        if *asset_id != Self::ASSET_ID {
            Err(xpallet_assets::Error::<T>::ActionNotAllowed)?
        }
        let fee = Self::btc_withdrawal_fee().saturated_into();
        let limit = WithdrawalLimit::<BalanceOf<T>> {
            minimal_withdrawal: fee * 3.saturated_into() / 2.saturated_into(),
            fee,
        };
        Ok(limit)
    }
}

impl<T: Trait> Module<T> {
    pub fn verify_btc_address(data: &[u8]) -> result::Result<Address, DispatchError> {
        let r = bs58::decode(data)
            .into_vec()
            .map_err(|_| Error::<T>::InvalidBase58)?;
        let addr = Address::from_layout(&r).map_err(|_| Error::<T>::InvalidAddr)?;
        Ok(addr)
    }

    fn apply_push_header(header: BtcHeader) -> DispatchResult {
        // current should not exist
        ensure_with_errorlog!(
            Self::headers(&header.hash()).is_none(),
            Error::<T>::ExistedHeader,
            "Header already exists|hash:{:}",
            header.hash(),
        );
        // prev header should exist, thus we reject orphan block
        let prev_info = Self::headers(header.previous_header_hash).ok_or_else(|| {
            native::error!(
                target: xpallet_support::RUNTIME_TARGET,
                "[check_prev_and_convert]|not find prev header|current header:{:?}",
                header
            );
            Error::<T>::PrevHeaderNotExisted
        })?;

        // convert btc header to self header info
        let header_info: BtcHeaderInfo = BtcHeaderInfo {
            header,
            height: prev_info.height + 1,
        };
        // check
        let c =
            header::HeaderVerifier::new::<T>(&header_info).map_err::<Error<T>, _>(Into::into)?;
        c.check::<T>()?;

        with_transaction_result(|| {
            // insert into storage
            let hash = header_info.header.hash();
            // insert valid header into storage
            Headers::insert(&hash, header_info.clone());
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
                let new_best_index = BtcHeaderIndex {
                    hash,
                    height: header_info.height,
                };
                // note update_confirmed_header would mutate other storage depend on BlockHashFor
                let confirmed_index = header::update_confirmed_header::<T>(&header_info);
                info!(
                    "[apply_push_header]|update to new height|height:{:}|hash:{:?}|confirm:{:?}",
                    header_info.height, hash, confirmed_index
                );
                // change new best index
                BestIndex::put(new_best_index);
            } else {
                // forked chain
                info!("[apply_push_header]|best index larger than this height|best height:{:}|this height{:}", best_index.height, header_info.height);
                header::check_confirmed_header::<T>(&header_info)?;
            };
            Self::deposit_event(RawEvent::InsertHeader(hash));
            Ok(())
        })
    }

    fn apply_push_transaction(tx: BtcRelayedTx, prev: Option<Transaction>) -> DispatchResult {
        let tx_hash = tx.raw.hash();
        let block_hash = tx.block_hash;
        let header_info = Module::<T>::headers(&tx.block_hash).ok_or_else(|| {
            error!(
                "[apply_push_transaction]|tx's block header must exist before|block_hash:{:}",
                block_hash
            );
            "tx's block header must exist before"
        })?;
        let merkle_root = header_info.header.merkle_root_hash;
        // verify, check merkle proof
        tx::validate_transaction::<T>(&tx, merkle_root, prev.as_ref())?;

        // ensure the tx should belong to the main chain, means should submit mainchain tx,
        // e.g. a tx may be packed in main chain block, and forked chain block, only submit main chain tx
        // could pass the verify.
        ensure!(
            Self::main_chain(&tx.block_hash).is_some(),
            Error::<T>::UnconfirmedTx
        );
        // if ConfirmedIndex not set, due to confirm height not beyond genesis height
        let confirmed = Self::confirmed_index().ok_or(Error::<T>::UnconfirmedTx)?;
        let height = header_info.height;
        if height > confirmed.height {
            error!("[apply_push_transaction]|receive an unconfirmed tx|tx hash:{:}|related block height:{:}|confirmed block height:{:}|hash:{:?}", tx_hash, height, confirmed.height, confirmed.hash);
            Err(Error::<T>::UnconfirmedTx)?;
        }
        // check whether replayed tx has been processed, just process failed and not processed tx;
        match Self::tx_state(&tx_hash) {
            None => { /* do nothing */ }
            Some(state) => {
                if state.result == BtcTxResult::Success {
                    error!("[apply_push_transaction]|reject processed tx|tx hash:{:}|type:{:?}|result:{:?}", tx_hash, state.tx_type, state.result);
                    Err(Error::<T>::ReplayedTx)?;
                }
            }
        }

        let state = tx::process_tx::<T>(tx.raw, prev)?;
        // set storage
        TxState::insert(&tx_hash, state);
        Self::deposit_event(RawEvent::ProcessTx(tx_hash, block_hash, state));
        match state.result {
            BtcTxResult::Success => Ok(()),
            BtcTxResult::Failed => Err(Error::<T>::ProcessTxFailed)?,
        }
    }
}
