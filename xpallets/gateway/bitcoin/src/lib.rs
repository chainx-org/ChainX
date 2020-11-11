// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
mod extractor;
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
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
    ensure,
    traits::{EnsureOrigin, UnixTime},
    weights::Pays,
};
use frame_system::{ensure_root, ensure_signed};

use orml_utilities::with_transaction_result;

// ChainX
use chainx_primitives::{AssetId, ReferralId};
use xp_gateway_common::AccountExtractor;
use xp_logging::{debug, error, info};
use xpallet_assets::{BalanceOf, Chain, ChainT, WithdrawalLimit};
use xpallet_gateway_common::{
    traits::{AddrBinding, ChannelBinding, TrusteeSession},
    trustees::bitcoin::BtcTrusteeAddrInfo,
};
use xpallet_support::{str, try_addr};

// light-bitcoin
#[cfg(feature = "std")]
pub use light_bitcoin::primitives::h256_rev;
pub use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    keys::Network as BtcNetwork,
    primitives::{hash_rev, Compact, H256, H264},
};
use light_bitcoin::{
    chain::Transaction,
    keys::{Address, DisplayLayout},
    serialization::{deserialize, Reader},
};

pub use self::extractor::OpReturnExtractor;
pub use self::types::{BtcAddress, BtcParams, BtcTxVerifier, BtcWithdrawalProposal};
use self::types::{
    BtcDepositCache, BtcHeaderIndex, BtcHeaderInfo, BtcRelayedTx, BtcRelayedTxInfo, BtcTxResult,
    BtcTxState,
};
use crate::trustee::get_trustee_address_pair;
use crate::tx::remove_pending_deposit;
use crate::weight_info::WeightInfo;

// syntactic sugar for native log.
#[macro_export]
macro_rules! native {
    ($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
        frame_support::debug::native::$level!(
            target: xp_logging::RUNTIME_TARGET,
            $patter $(, $values)*
        )
    };
}

pub trait Trait: xpallet_assets::Trait + xpallet_gateway_records::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type UnixTime: UnixTime;
    type AccountExtractor: AccountExtractor<Self::AccountId, ReferralId>;
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
        ExistingHeader,
        /// Can't find previous header
        PrevHeaderNotExisted,
        /// Cannot deserialize the header or tx vec
        DeserializeErr,
        ///
        BadMerkleProof,
        /// The tx is not yet confirmed, i.e, the block of which is not confirmed.
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
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>
    {
        /// A Bitcoin header was validated and inserted. [btc_header_hash]
        HeaderInserted(H256),
        /// A Bitcoin transaction was processed. [tx_hash, block_hash, tx_state]
        TxProcessed(H256, H256, BtcTxState),
        /// An account deposited some token. [tx_hash, who, amount]
        Deposited(H256, AccountId, Balance),
        /// A list of withdrawal applications were processed successfully. [tx_hash, withdrawal_ids, total_withdrawn]
        Withdrawn(H256, Vec<u32>, Balance),
        /// A new record of unclaimed deposit. [tx_hash, btc_address]
        UnclaimedDeposit(H256, BtcAddress),
        /// A unclaimed deposit record was removed. [depositor, deposit_amount, tx_hash, btc_address]
        PendingDepositRemoved(AccountId, Balance, H256, BtcAddress),
        /// A new withdrawal proposal was created. [proposer, withdrawal_ids]
        WithdrawalProposalCreated(AccountId, Vec<u32>),
        /// A trustee voted/vetoed a withdrawal proposal. [trustee, vote_status]
        WithdrawalProposalVoted(AccountId, bool),
        /// A withdrawal proposal was dropped. [reject_count, total_count, withdrawal_ids]
        WithdrawalProposalDropped(u32, u32, Vec<u32>),
        /// The proposal has been processed successfully and is waiting for broadcasting. [tx_hash]
        WithdrawalProposalCompleted(H256),
        /// A fatal error happened during the withdrwal process. [tx_hash, proposal_hash]
        WithdrawalFatalErr(H256, H256),
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
        pub MainChain get(fn main_chain): map hasher(identity) H256 => bool;
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
        config(genesis_trustees): Vec<T::AccountId>;
        build(|config| {
            let genesis_hash = config.genesis_hash;
            let (genesis_header, genesis_height) = config.genesis_info;
            let genesis_index = BtcHeaderIndex {
                hash: genesis_hash,
                height: genesis_height,
            };
            let header_info = BtcHeaderInfo {
                header: genesis_header,
                height: genesis_height,
            };
            // would ignore check for bitcoin testnet
            #[cfg(not(test))] {
            if let BtcNetwork::Mainnet = config.network_id {
                if genesis_index.height % config.params_info.retargeting_interval() != 0 {
                    panic!("Block #{} should start from a changed difficulty block", genesis_index.height);
                }
            }
            }

            Headers::insert(&genesis_hash, header_info);
            BlockHashFor::insert(&genesis_index.height, vec![genesis_hash]);
            MainChain::insert(&genesis_hash, true);
            BestIndex::put(genesis_index);

            // init trustee (not this action should ha)
            if !config.genesis_trustees.is_empty() {
                T::TrusteeSessionProvider::genesis_trustee(Module::<T>::chain(), &config.genesis_trustees);
            }
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
            let from = ensure_signed(origin)?;
            let header: BtcHeader = deserialize(header.as_slice()).map_err(|_| Error::<T>::DeserializeErr)?;
            debug!("[push_header] from:{:?}, header:{:?}", from, header);

            Self::apply_push_header(header)?;

            let post_info = PostDispatchInfo {
                actual_weight: Some(Zero::zero()),
                pays_fee: Pays::No,
            };
            Ok(post_info)
        }

        /// if use `RelayTx` struct would export in metadata, cause complex in front-end
        #[weight = <T as Trait>::WeightInfo::push_transaction()]
        pub fn push_transaction(
            origin,
            raw_tx: Vec<u8>,
            relayed_info: BtcRelayedTxInfo,
            prev_tx: Option<Vec<u8>>
        ) -> DispatchResultWithPostInfo {
            let _from = ensure_signed(origin)?;
            let raw_tx = Self::deserialize_tx(raw_tx.as_slice())?;
            let prev = if let Some(prev) = prev_tx {
                Some(Self::deserialize_tx(prev.as_slice())?)
            } else {
                None
            };
            let relay_tx = relayed_info.into_relayed_tx(raw_tx);
            native!(debug, "[push_transaction] from:{:?}, relay_tx:{:?}, prev:{:?}", _from, relay_tx, prev);

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

            let tx = Self::deserialize_tx(tx.as_slice())?;
            native!(debug, "[create_withdraw_tx] from:{:?}, withdrawal list:{:?}, tx:{:?}", from, withdrawal_id_list, tx);

            Self::apply_create_withdraw(from, tx, withdrawal_id_list)?;
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
                Some(Self::deserialize_tx(raw_tx.as_slice())?)
            } else {
                None
            };
            native!(debug, "[sign_withdraw_tx] from:{:?}, vote_tx:{:?}", from, tx);

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
                info!("[remove_pending] Release pending deposit directly, not deposit to someone, addr:{:?}", str!(&addr));
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
            let tx = Self::deserialize_tx(tx.as_slice())?;
            native!(debug, "[force_replace_proposal_tx] new_tx:{:?}", tx);
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
    const ASSET_ID: AssetId = xp_protocol::X_BTC;

    fn chain() -> Chain {
        Chain::Bitcoin
    }

    fn check_addr(addr: &[u8], _: &[u8]) -> DispatchResult {
        // this addr is base58 addr
        let address = Self::verify_btc_address(addr).map_err(|err| {
            error!(
                "[verify_btc_address] Verify failed, error:{:?}, source addr:{:?}",
                err,
                try_addr!(addr)
            );
            err
        })?;

        match get_trustee_address_pair::<T>() {
            Ok((hot_addr, cold_addr)) => {
                // do not allow withdraw from trustee address
                if address == hot_addr || address == cold_addr {
                    return Err(Error::<T>::InvalidAddress.into());
                }
            }
            Err(err) => {
                error!("[check_addr] Can not get trustee addr:{:?}", err);
            }
        }

        Ok(())
    }

    fn withdrawal_limit(
        asset_id: &AssetId,
    ) -> result::Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
        if *asset_id != Self::ASSET_ID {
            return Err(xpallet_assets::Error::<T>::ActionNotAllowed.into());
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

    /// Helper function for deserializing the slice of raw tx.
    #[inline]
    fn deserialize_tx(input: &[u8]) -> result::Result<Transaction, Error<T>> {
        deserialize(Reader::new(input)).map_err(|_| Error::<T>::DeserializeErr)
    }

    fn apply_push_header(header: BtcHeader) -> DispatchResult {
        // current should not exist
        if Self::headers(&header.hash()).is_some() {
            error!(
                "[apply_push_header] The BTC header already exists, hash:{:?}",
                header.hash()
            );
            return Err(Error::<T>::ExistingHeader.into());
        }
        // prev header should exist, thus we reject orphan block
        let prev_info = Self::headers(header.previous_header_hash).ok_or_else(|| {
            native!(
                error,
                "[check_prev_and_convert] Can not find prev header, current header:{:?}",
                header
            );
            Error::<T>::PrevHeaderNotExisted
        })?;

        // convert btc header to self header info
        let header_info = BtcHeaderInfo {
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
                    v.push(hash);
                }
            });

            debug!(
                "[apply_push_header] Verify successfully, insert header to storage [height:{}, hash:{:?}, all hashes of the height:{:?}]",
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
                    "[apply_push_header] Update new height:{}, hash:{:?}, confirm:{:?}",
                    header_info.height, hash, confirmed_index
                );
                // change new best index
                BestIndex::put(new_best_index);
            } else {
                // forked chain
                info!(
                    "[apply_push_header] Best index {} larger than this height {}",
                    best_index.height, header_info.height
                );
                header::check_confirmed_header::<T>(&header_info)?;
            };
            Self::deposit_event(Event::<T>::HeaderInserted(hash));
            Ok(())
        })
    }

    fn apply_push_transaction(tx: BtcRelayedTx, prev: Option<Transaction>) -> DispatchResult {
        let tx_hash = tx.raw.hash();
        let block_hash = tx.block_hash;
        let header_info = Module::<T>::headers(&tx.block_hash).ok_or_else(|| {
            error!(
                "[apply_push_transaction] Tx's block header ({:?}) must exist before",
                block_hash
            );
            "Tx's block header must already exist"
        })?;
        let merkle_root = header_info.header.merkle_root_hash;
        // verify, check merkle proof
        tx::validate_transaction::<T>(&tx, merkle_root, prev.as_ref())?;

        // ensure the tx should belong to the main chain, means should submit mainchain tx,
        // e.g. a tx may be packed in main chain block, and forked chain block, only submit main chain tx
        // could pass the verify.
        ensure!(Self::main_chain(&tx.block_hash), Error::<T>::UnconfirmedTx);
        // if ConfirmedIndex not set, due to confirm height not beyond genesis height
        let confirmed = Self::confirmed_index().ok_or(Error::<T>::UnconfirmedTx)?;
        let height = header_info.height;
        if height > confirmed.height {
            error!(
                "[apply_push_transaction] Receive an unconfirmed tx (height:{}, hash:{:?}), confirmed index (height:{}, hash:{:?})", 
                height, tx_hash, confirmed.height, confirmed.hash
            );
            return Err(Error::<T>::UnconfirmedTx.into());
        }
        // check whether replayed tx has been processed, just process failed and not processed tx;
        match Self::tx_state(&tx_hash) {
            None => { /* do nothing */ }
            Some(state) => {
                if state.result == BtcTxResult::Success {
                    error!(
                        "[apply_push_transaction] Reject processed tx (hash:{:?}, type:{:?}, result:{:?})", 
                        tx_hash, state.tx_type, state.result
                    );
                    return Err(Error::<T>::ReplayedTx.into());
                }
            }
        }

        let state = tx::process_tx::<T>(tx.raw, prev)?;
        TxState::insert(&tx_hash, state);
        Self::deposit_event(Event::<T>::TxProcessed(tx_hash, block_hash, state));
        match state.result {
            BtcTxResult::Success => Ok(()),
            BtcTxResult::Failed => Err(Error::<T>::ProcessTxFailed.into()),
        }
    }
}
