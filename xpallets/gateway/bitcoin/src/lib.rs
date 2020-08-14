// Copyright 2018-2019 Chainpool.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

pub mod header;
mod tests;
pub mod trustee;
pub mod tx;
mod types;

// Substrate
use sp_runtime::{traits::Zero, SaturatedConversion};
use sp_std::{prelude::*, result};

use frame_support::{
    debug::native,
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
    traits::Currency,
};
use frame_system::{ensure_root, ensure_signed};

// ChainX
use chainx_primitives::{AddrStr, AssetId};
use xpallet_assets::{Chain, ChainT, WithdrawalLimit};
use xpallet_gateway_common::{
    traits::{AddrBinding, ChannelBinding, Extractable, TrusteeSession},
    trustees::bitcoin::BtcTrusteeAddrInfo,
};
use xpallet_support::{
    base58, debug, ensure_with_errorlog, error, info, str, try_addr, RUNTIME_TARGET,
};

// light-bitcoin
use btc_chain::Transaction;
use btc_keys::{Address, DisplayLayout};
use btc_ser::{deserialize, Reader};
// re-export
pub use btc_chain::BlockHeader as BtcHeader;
pub use btc_keys::Network as BtcNetwork;
#[cfg(feature = "std")]
pub use btc_primitives::h256_conv_endian_from_str;
pub use btc_primitives::{Compact, H256, H264};

pub use self::types::{BtcAddress, BtcParams, BtcTxVerifier, BtcWithdrawalProposal};
use self::types::{
    BtcDepositCache, BtcHeaderIndex, BtcHeaderInfo, BtcRelayedTx, BtcRelayedTxInfo, BtcTxResult,
    BtcTxState,
};
use crate::trustee::get_trustee_address_pair;
use crate::tx::remove_pending_deposit;
use frame_support::traits::EnsureOrigin;

pub type BalanceOf<T> = <<T as xpallet_assets::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait:
    frame_system::Trait
    + pallet_timestamp::Trait
    + xpallet_assets::Trait
    + xpallet_gateway_records::Trait
{
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type AccountExtractor: Extractable<Self::AccountId>;
    type TrusteeSessionProvider: TrusteeSession<Self::AccountId, BtcTrusteeAddrInfo>;
    type TrusteeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;
    type Channel: ChannelBinding<Self::AccountId>;
    type AddrBinding: AddrBinding<Self::AccountId, BtcAddress>;
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
        ///
        MismatchedTx,
        ///
        InvalidAddress,
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
        ///
        NotTrustee,
        ///
        DuplicatedKeys,
        ///
        GenerateMultisigFailed,
        ///
        InvalidTrusteeCounts,
        ///
        WroungWithdrawalCount,
        ///
        InvalidSigCount,
        ///
        RejectSig,
        ///
        NoProposal,
        ///
        InvalidProposal,
        ///
        NotFinishProposal,
        ///
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
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XGatewayBitcoin {
        /// best header info
        pub BestIndex get(fn best_index): BtcHeaderIndex;
        /// confirmed header info
        pub ConfirmedHeader get(fn confirmed_header): BtcHeaderIndex;
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
            ConfirmedHeader::put(genesis_index);

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
        #[weight = 0]
        pub fn push_header(origin, header: Vec<u8>) -> DispatchResult {
            let _from = ensure_signed(origin)?;
            let header: BtcHeader = deserialize(header.as_slice()).map_err(|_| Error::<T>::DeserializeErr)?;
            debug!("[push_header]|from:{:?}|header:{:?}", _from, header);

            Self::apply_push_header(header)?;
            Ok(())
        }

        /// if use `RelayTx` struct would export in metadata, cause complex in front-end
        #[weight = 0]
        pub fn push_transaction(origin, raw_tx: Vec<u8>, relayed_info: BtcRelayedTxInfo, prev_tx: Option<Vec<u8>>) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let raw_tx: Transaction = deserialize(Reader::new(raw_tx.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
            let prev = if let Some(prev) = prev_tx {
                let prev: Transaction = deserialize(Reader::new(prev.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
                Some(prev)
            } else {
                None
            };
            let relay_tx = relayed_info.into_relayed_tx(raw_tx);
            native::debug!(
                target: RUNTIME_TARGET,
                "[push_transaction]|from:{:?}|relay_tx:{:?}|prev:{:?}", from, relay_tx, prev
            );

            Self::apply_push_transaction(relay_tx, prev)?;

            let post_info = PostDispatchInfo {
                actual_weight: Some(Zero::zero()),
            };
            Ok(post_info)
        }

        #[weight = 0]
        pub fn create_withdraw_tx(origin, withdrawal_id_list: Vec<u32>, tx: Vec<u8>) -> DispatchResult {
            let from = ensure_signed(origin)?;
            // commiter must in trustee list
            Self::ensure_trustee(&from)?;

            let tx: Transaction = deserialize(Reader::new(tx.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
            native::debug!(target: RUNTIME_TARGET, "[create_withdraw_tx]|from:{:?}|withdrawal list:{:?}|tx:{:?}", from, withdrawal_id_list, tx);

            Self::apply_create_withdraw(from, tx, withdrawal_id_list.clone())?;
            Ok(())
        }

        #[weight = 0]
        pub fn sign_withdraw_tx(origin, tx: Option<Vec<u8>>) -> DispatchResult {
            let from = ensure_signed(origin)?;
            Self::ensure_trustee(&from)?;

            let tx = if let Some(raw_tx) = tx {
                let tx: Transaction = deserialize(Reader::new(raw_tx.as_slice())).map_err(|_| Error::<T>::DeserializeErr)?;
                Some(tx)
            } else {
                None
            };
            native::debug!(target: RUNTIME_TARGET, "[sign_withdraw_tx]|from:{:?}|vote_tx:{:?}", from, tx);

            Self::apply_sig_withdraw(from, tx)?;
            Ok(())
        }

        /// Dangerous! Be careful to set BestIndex
        #[weight = 0]
        pub fn set_best_index(origin, index: BtcHeaderIndex) -> DispatchResult {
            ensure_root(origin)?;
            BestIndex::put(index);
            Ok(())
        }
        /// Dangerous! Be careful to set ConfirmedIndex
        #[weight = 0]
        pub fn set_confirmed_index(origin, index: BtcHeaderIndex) -> DispatchResult {
            ensure_root(origin)?;
            ConfirmedHeader::put(index);
            Ok(())
        }

        #[weight = 0]
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

        #[weight = 0]
        pub fn remove_proposal(origin) -> DispatchResult {
            ensure_root(origin)?;
            WithdrawalProposal::<T>::kill();
            Ok(())
        }

        #[weight = 0]
        pub fn set_btc_withdrawal_fee(origin, #[compact] fee: u64) -> DispatchResult {
            T::TrusteeOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;
            BtcWithdrawalFee::put(fee);
            Ok(())
        }

        #[weight = 0]
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

        let (hot_addr, cold_addr) = get_trustee_address_pair::<T>()?;
        if address == hot_addr || address == cold_addr {
            Err(Error::<T>::InvalidAddress)?;
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
        let r = base58::from(data).map_err(|_| Error::<T>::InvalidBase58)?;
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
                target: RUNTIME_TARGET,
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

            let confirmed_index = header::update_confirmed_header::<T>(&header_info);
            info!(
                "[apply_push_header]|update to new height|height:{:}|hash:{:?}|confirm:{:?}",
                header_info.height, hash, confirmed_index
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

        let confirmed = Self::confirmed_header();
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
        Ok(())
    }
}
