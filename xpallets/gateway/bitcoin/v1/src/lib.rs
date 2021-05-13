// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! this module is for btc-bridge

#![cfg_attr(not(feature = "std"), no_std)]

mod header;
pub mod trustee;
mod tx;
mod types;
pub mod weights;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

use orml_utilities::with_transaction_result;

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

use chainx_primitives::{AssetId, ReferralId};
use xp_gateway_common::AccountExtractor;
use xp_logging::{debug, error, info};
use xpallet_assets::{BalanceOf, Chain, ChainT, WithdrawalLimit};
use xpallet_gateway_common::{
    traits::{AddressBinding, ReferralBinding, TrusteeSession},
    trustees::bitcoin::BtcTrusteeAddrInfo,
};
use xpallet_support::try_addr;

pub use self::types::{BtcAddress, BtcParams, BtcTxVerifier, BtcWithdrawalProposal};
pub use self::weights::WeightInfo;
use self::{
    trustee::{get_current_trustee_address_pair, get_last_trustee_address_pair},
    tx::remove_pending_deposit,
    types::{
        BtcDepositCache, BtcHeaderIndex, BtcHeaderInfo, BtcRelayedTx, BtcRelayedTxInfo,
        BtcTxResult, BtcTxState,
    },
};

pub use pallet::*;

// syntactic sugar for native log.
#[macro_export]
macro_rules! native {
    ($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
        frame_support::log::$level!(
            target: xp_logging::RUNTIME_TARGET,
            $patter $(, $values)*
        )
    };
}

#[frame_support::pallet]
pub mod pallet {
    use sp_std::marker::PhantomData;

    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::UnixTime};
    use frame_system::pallet_prelude::*;

    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

    #[pallet::config]
    pub trait Config<I: 'static = ()>:
        frame_system::Config + xpallet_assets::Config + xpallet_gateway_records::Config
    {
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;
        type UnixTime: UnixTime;
        type AccountExtractor: AccountExtractor<Self::AccountId, ReferralId>;
        type TrusteeSessionProvider: TrusteeSession<Self::AccountId, BtcTrusteeAddrInfo>;
        type TrusteeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;
        type ReferralBinding: ReferralBinding<Self::AccountId>;
        type AddressBinding: AddressBinding<Self::AccountId, BtcAddress>;
        type WeightInfo: WeightInfo;
    }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        /// if use `BtcHeader` struct would export in metadata, cause complex in front-end
        #[pallet::weight(<T as Config<I>>::WeightInfo::push_header())]
        pub fn push_header(origin: OriginFor<T>, header: Vec<u8>) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let header: BtcHeader =
                deserialize(header.as_slice()).map_err(|_| Error::<T, I>::DeserializeErr)?;
            debug!("[push_header] from:{:?}, header:{:?}", from, header);

            Self::apply_push_header(header)?;

            // Relayer does not pay a fee.
            Ok(Pays::No.into())
        }

        /// if use `RelayTx` struct would export in metadata, cause complex in front-end
        #[pallet::weight(<T as Config<I>>::WeightInfo::push_transaction())]
        pub fn push_transaction(
            origin: OriginFor<T>,
            raw_tx: Vec<u8>,
            relayed_info: BtcRelayedTxInfo,
            prev_tx: Option<Vec<u8>>,
        ) -> DispatchResultWithPostInfo {
            let _from = ensure_signed(origin)?;
            let raw_tx = Self::deserialize_tx(raw_tx.as_slice())?;
            let prev_tx = if let Some(prev_tx) = prev_tx {
                Some(Self::deserialize_tx(prev_tx.as_slice())?)
            } else {
                None
            };
            let relay_tx = relayed_info.into_relayed_tx(raw_tx);
            native!(
                debug,
                "[push_transaction] from:{:?}, relay_tx:{:?}, prev_tx:{:?}",
                _from,
                relay_tx,
                prev_tx
            );

            Self::apply_push_transaction(relay_tx, prev_tx)?;

            Ok(Pays::No.into())
        }

        /// Trustee create a proposal for a withdrawal list. `tx` is the proposal withdrawal transaction.
        /// The `tx` would have a sign for current creator or do not have sign. if creator do not sign
        /// for this transaction, he could do `sign_withdraw_tx` later.
        #[pallet::weight(<T as Config<I>>::WeightInfo::create_withdraw_tx())]
        pub fn create_withdraw_tx(
            origin: OriginFor<T>,
            withdrawal_id_list: Vec<u32>,
            tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            // committer must be in the trustee list
            Self::ensure_trustee(&from)?;

            let tx = Self::deserialize_tx(tx.as_slice())?;
            native!(
                debug,
                "[create_withdraw_tx] from:{:?}, withdrawal list:{:?}, tx:{:?}",
                from,
                withdrawal_id_list,
                tx
            );

            Self::apply_create_withdraw(from, tx, withdrawal_id_list)?;
            Ok(().into())
        }

        /// Trustees sign a withdrawal proposal. If `tx` is None, means this trustee vote to reject
        /// this proposal. If `tx` is Some(), the inner part must be a valid transaction with this
        /// trustee signature.
        #[pallet::weight(<T as Config<I>>::WeightInfo::sign_withdraw_tx())]
        pub fn sign_withdraw_tx(
            origin: OriginFor<T>,
            tx: Option<Vec<u8>>,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            Self::ensure_trustee(&from)?;

            let tx = if let Some(raw_tx) = tx {
                Some(Self::deserialize_tx(raw_tx.as_slice())?)
            } else {
                None
            };
            native!(
                debug,
                "[sign_withdraw_tx] from:{:?}, vote_tx:{:?}",
                from,
                tx
            );

            Self::apply_sig_withdraw(from, tx)?;
            Ok(().into())
        }

        /// Dangerous! Be careful to set BestIndex
        #[pallet::weight(<T as Config<I>>::WeightInfo::set_best_index())]
        pub fn set_best_index(
            origin: OriginFor<T>,
            index: BtcHeaderIndex,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            BestIndex::<T, I>::put(index);
            Ok(().into())
        }

        /// Dangerous! Be careful to set ConfirmedIndex
        #[pallet::weight(<T as Config<I>>::WeightInfo::set_confirmed_index())]
        pub fn set_confirmed_index(
            origin: OriginFor<T>,
            index: BtcHeaderIndex,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ConfirmedIndex::<T, I>::put(index);
            Ok(().into())
        }

        /// Allow root or trustees could remove pending deposits for an address and decide whether
        /// deposit to an account id. if pass `None` to `who`, would just remove pendings, if pass
        /// Some, would deposit to this account id.
        #[pallet::weight(<T as Config<I>>::WeightInfo::remove_pending())]
        pub fn remove_pending(
            origin: OriginFor<T>,
            addr: BtcAddress,
            who: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            T::TrusteeOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;

            if let Some(w) = who {
                remove_pending_deposit::<T, I>(&addr, &w);
            } else {
                info!("[remove_pending] Release pending deposit directly, not deposit to someone, addr:{:?}", try_addr(&addr));
                PendingDeposits::<T, I>::remove(&addr);
            }
            Ok(().into())
        }

        /// Dangerous! remove current withdrawal proposal directly. Please check business logic before
        /// do this operation.
        #[pallet::weight(<T as Config<I>>::WeightInfo::remove_proposal())]
        pub fn remove_proposal(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            WithdrawalProposal::<T, I>::kill();
            Ok(().into())
        }

        /// Dangerous! force replace current withdrawal proposal transaction. Please check business
        /// logic before do this operation. Must make sure current proposal transaction is invalid
        /// (e.g. when created a proposal, the inputs are not in double spend state, but after other
        /// trustees finish signing, the inputs are in double spend due other case. Thus could create
        /// a new valid transaction which outputs same to current proposal to replace current proposal
        /// transaction.)
        #[pallet::weight(<T as Config<I>>::WeightInfo::force_replace_proposal_tx())]
        pub fn force_replace_proposal_tx(
            origin: OriginFor<T>,
            tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            T::TrusteeOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;
            let tx = Self::deserialize_tx(tx.as_slice())?;
            native!(debug, "[force_replace_proposal_tx] new_tx:{:?}", tx);
            Self::force_replace_withdraw_tx(tx)?;
            Ok(().into())
        }

        /// Set bitcoin withdrawal fee
        #[pallet::weight(<T as Config<I>>::WeightInfo::set_btc_withdrawal_fee())]
        pub fn set_btc_withdrawal_fee(
            origin: OriginFor<T>,
            #[pallet::compact] fee: u64,
        ) -> DispatchResultWithPostInfo {
            T::TrusteeOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;
            BtcWithdrawalFee::<T, I>::put(fee);
            Ok(().into())
        }

        /// Set bitcoin deposit limit
        #[pallet::weight(<T as Config<I>>::WeightInfo::set_btc_deposit_limit())]
        pub fn set_btc_deposit_limit(
            origin: OriginFor<T>,
            #[pallet::compact] value: u64,
        ) -> DispatchResultWithPostInfo {
            T::TrusteeOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;
            BtcMinDeposit::<T, I>::put(value);
            Ok(().into())
        }
    }

    /// Error for the XBridge Bitcoin module
    #[pallet::error]
    pub enum Error<T, I = ()> {
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
        /// already vote for this withdrawal proposal
        DuplicateVote,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        /// A Bitcoin header was validated and inserted. [btc_header_hash]
        HeaderInserted(H256),
        /// A Bitcoin transaction was processed. [tx_hash, block_hash, tx_state]
        TxProcessed(H256, H256, BtcTxState),
        /// An account deposited some token. [tx_hash, who, amount]
        Deposited(H256, T::AccountId, BalanceOf<T>),
        /// A list of withdrawal applications were processed successfully. [tx_hash, withdrawal_ids, total_withdrawn]
        Withdrawn(H256, Vec<u32>, BalanceOf<T>),
        /// A new record of unclaimed deposit. [tx_hash, btc_address]
        UnclaimedDeposit(H256, BtcAddress),
        /// A unclaimed deposit record was removed. [depositor, deposit_amount, tx_hash, btc_address]
        PendingDepositRemoved(T::AccountId, BalanceOf<T>, H256, BtcAddress),
        /// A new withdrawal proposal was created. [proposer, withdrawal_ids]
        WithdrawalProposalCreated(T::AccountId, Vec<u32>),
        /// A trustee voted/vetoed a withdrawal proposal. [trustee, vote_status]
        WithdrawalProposalVoted(T::AccountId, bool),
        /// A withdrawal proposal was dropped. [reject_count, total_count, withdrawal_ids]
        WithdrawalProposalDropped(u32, u32, Vec<u32>),
        /// The proposal has been processed successfully and is waiting for broadcasting. [tx_hash]
        WithdrawalProposalCompleted(H256),
        /// A fatal error happened during the withdrwal process. [tx_hash, proposal_hash]
        WithdrawalFatalErr(H256, H256),
    }

    /// best header info
    #[pallet::storage]
    #[pallet::getter(fn best_index)]
    pub(crate) type BestIndex<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BtcHeaderIndex, ValueQuery>;

    /// confirmed header info
    #[pallet::storage]
    #[pallet::getter(fn confirmed_index)]
    pub(crate) type ConfirmedIndex<T: Config<I>, I: 'static = ()> = StorageValue<_, BtcHeaderIndex>;

    /// block hash list for a height, include forked header hash
    #[pallet::storage]
    #[pallet::getter(fn block_hash_for)]
    pub(crate) type BlockHashFor<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Twox64Concat, u32, Vec<H256>, ValueQuery>;

    /// mark this blockhash is in mainchain
    #[pallet::storage]
    #[pallet::getter(fn main_chain)]
    pub(crate) type MainChain<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Identity, H256, bool, ValueQuery>;

    /// all valid blockheader (include forked blockheader)
    #[pallet::storage]
    #[pallet::getter(fn headers)]
    pub(crate) type Headers<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Identity, H256, BtcHeaderInfo>;

    /// mark tx has been handled, in case re-handle this tx, and log handle result
    #[pallet::storage]
    #[pallet::getter(fn tx_state)]
    pub(crate) type TxState<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Identity, H256, BtcTxState>;

    /// unclaimed deposit info, addr => tx_hash, btc value,
    #[pallet::storage]
    #[pallet::getter(fn pending_deposits)]
    pub(crate) type PendingDeposits<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, BtcAddress, Vec<BtcDepositCache>, ValueQuery>;

    /// withdrawal tx outs for account, tx_hash => outs ( out index => withdrawal account )
    #[pallet::storage]
    #[pallet::getter(fn withdrawal_proposal)]
    pub(crate) type WithdrawalProposal<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BtcWithdrawalProposal<T::AccountId>>;

    /// get GenesisInfo (header, height)
    #[pallet::storage]
    #[pallet::getter(fn genesis_info)]
    pub(crate) type GenesisInfo<T: Config<I>, I: 'static = ()> =
        StorageValue<_, (BtcHeader, u32), ValueQuery>;

    /// get ParamsInfo from genesis_config
    #[pallet::storage]
    #[pallet::getter(fn params_info)]
    pub(crate) type ParamsInfo<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BtcParams, ValueQuery>;

    ///  NetworkId for testnet or mainnet
    #[pallet::storage]
    #[pallet::getter(fn network_id)]
    pub(crate) type NetworkId<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BtcNetwork, ValueQuery>;

    /// get ConfirmationNumber from genesis_config
    #[pallet::storage]
    #[pallet::getter(fn confirmation_number)]
    pub(crate) type ConfirmationNumber<T: Config<I>, I: 'static = ()> =
        StorageValue<_, u32, ValueQuery>;

    /// get BtcWithdrawalFee from genesis_config
    #[pallet::storage]
    #[pallet::getter(fn btc_withdrawal_fee)]
    pub(crate) type BtcWithdrawalFee<T: Config<I>, I: 'static = ()> =
        StorageValue<_, u64, ValueQuery>;
    #[pallet::type_value]
    pub fn DefaultForMinDeposit<T: Config<I>, I: 'static>() -> u64 {
        1 * 100000
    }

    /// min deposit value limit, default is 10w sotashi(0.001 BTC)
    #[pallet::storage]
    #[pallet::getter(fn btc_min_deposit)]
    pub(crate) type BtcMinDeposit<T: Config<I>, I: 'static = ()> =
        StorageValue<_, u64, ValueQuery, DefaultForMinDeposit<T, I>>;

    /// max withdraw account count in bitcoin withdrawal transaction
    #[pallet::storage]
    #[pallet::getter(fn max_withdrawal_count)]
    pub(crate) type MaxWithdrawalCount<T: Config<I>, I: 'static = ()> =
        StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn verifier)]
    pub(crate) type Verifier<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BtcTxVerifier, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
        pub genesis_hash: H256,
        pub genesis_info: (BtcHeader, u32),
        pub genesis_trustees: Vec<T::AccountId>,
        pub params_info: BtcParams,
        pub network_id: BtcNetwork,
        pub confirmation_number: u32,
        pub btc_withdrawal_fee: u64,
        pub max_withdrawal_count: u32,
        pub verifier: BtcTxVerifier,
        pub(crate) _marker: sp_std::marker::PhantomData<I>,
    }

    #[cfg(feature = "std")]
    impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
        fn default() -> Self {
            Self {
                genesis_hash: Default::default(),
                genesis_info: Default::default(),
                genesis_trustees: Default::default(),
                params_info: Default::default(),
                network_id: Default::default(),
                confirmation_number: Default::default(),
                btc_withdrawal_fee: Default::default(),
                max_withdrawal_count: Default::default(),
                verifier: Default::default(),
                _marker: sp_std::marker::PhantomData::<I>,
            }
        }
    }

    #[pallet::genesis_build]
    #[cfg(feature = "std")]
    impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
        fn build(&self) {
            let genesis_hash = &self.genesis_hash.clone();
            let (genesis_header, genesis_height) = &self.genesis_info.clone();
            let genesis_index = BtcHeaderIndex {
                hash: *genesis_hash,
                height: genesis_height.clone(),
            };
            let header_info = BtcHeaderInfo {
                header: *genesis_header,
                height: *genesis_height,
            };

            Headers::<T, I>::insert(&self.genesis_hash.clone(), header_info);
            BlockHashFor::<T, I>::insert(&genesis_index.height, vec![genesis_hash]);
            MainChain::<T, I>::insert(&genesis_hash, true);
            BestIndex::<T, I>::put(genesis_index);
            GenesisInfo::<T, I>::put(self.genesis_info);
            ParamsInfo::<T, I>::put(self.params_info);
            NetworkId::<T, I>::put(self.network_id);
            ConfirmationNumber::<T, I>::put(self.confirmation_number);
            BtcWithdrawalFee::<T, I>::put(self.btc_withdrawal_fee);
            MaxWithdrawalCount::<T, I>::put(self.max_withdrawal_count);
            Verifier::<T, I>::put(self.verifier);

            if !self.genesis_trustees.is_empty() {
                T::TrusteeSessionProvider::genesis_trustee(
                    Pallet::<T, I>::chain(),
                    &self.genesis_trustees,
                );
            }
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        pub fn verify_btc_address(data: &[u8]) -> Result<Address, DispatchError> {
            let r = bs58::decode(data)
                .into_vec()
                .map_err(|_| Error::<T, I>::InvalidBase58)?;
            let addr = Address::from_layout(&r).map_err(|_| Error::<T, I>::InvalidAddr)?;
            Ok(addr)
        }

        /// Helper function for deserializing the slice of raw tx.
        #[inline]
        pub(crate) fn deserialize_tx(input: &[u8]) -> Result<Transaction, Error<T, I>> {
            deserialize(Reader::new(input)).map_err(|_| Error::<T, I>::DeserializeErr)
        }

        pub(crate) fn apply_push_header(header: BtcHeader) -> DispatchResult {
            // current should not exist
            if Self::headers(&header.hash()).is_some() {
                error!(
                    "[apply_push_header] The BTC header already exists, hash:{:?}",
                    header.hash()
                );
                return Err(Error::<T, I>::ExistingHeader.into());
            }
            // prev header should exist, thus we reject orphan block
            let prev_info = Self::headers(header.previous_header_hash).ok_or_else(|| {
                native!(
                    error,
                    "[check_prev_and_convert] Can not find prev header, current header:{:?}",
                    header
                );
                Error::<T, I>::PrevHeaderNotExisted
            })?;

            // convert btc header to self header info
            let header_info = BtcHeaderInfo {
                header,
                height: prev_info.height + 1,
            };
            // verify header
            let header_verifier = header::HeaderVerifier::new::<T, I>(&header_info);
            header_verifier.check::<T, I>()?;

            with_transaction_result(|| {
                // insert into storage
                let hash = header_info.header.hash();
                // insert valid header into storage
                Headers::<T, I>::insert(&hash, header_info.clone());
                // storage height => block list (contains forked header hash)
                BlockHashFor::<T, I>::mutate(header_info.height, |v| {
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
                    // note update_confirmed_header would mutate other storage depend on BlockHashFor
                    let confirmed_index = header::update_confirmed_header::<T, I>(&header_info);
                    info!(
                        "[apply_push_header] Update new height:{}, hash:{:?}, confirm:{:?}",
                        header_info.height, hash, confirmed_index
                    );

                    // new best index
                    let new_best_index = BtcHeaderIndex {
                        hash,
                        height: header_info.height,
                    };
                    BestIndex::<T, I>::put(new_best_index);
                } else {
                    // forked chain
                    info!(
                        "[apply_push_header] Best index {} larger than this height {}",
                        best_index.height, header_info.height
                    );
                    header::check_confirmed_header::<T, I>(&header_info)?;
                };
                Self::deposit_event(Event::<T, I>::HeaderInserted(hash));
                Ok(())
            })
        }

        pub(crate) fn apply_push_transaction(
            tx: BtcRelayedTx,
            prev_tx: Option<Transaction>,
        ) -> DispatchResult {
            let tx_hash = tx.raw.hash();
            let block_hash = tx.block_hash;
            let header_info = Pallet::<T, I>::headers(&tx.block_hash).ok_or_else(|| {
                error!(
                    "[apply_push_transaction] Tx's block header ({:?}) must exist before",
                    block_hash
                );
                "Tx's block header must already exist"
            })?;
            let merkle_root = header_info.header.merkle_root_hash;
            // verify, check merkle proof
            tx::validate_transaction::<T, I>(&tx, merkle_root, prev_tx.as_ref())?;

            // ensure the tx should belong to the main chain, means should submit main chain tx,
            // e.g. a tx may be packed in main chain block, and forked chain block, only submit main chain tx
            // could pass the verify.
            ensure!(
                Self::main_chain(&tx.block_hash),
                Error::<T, I>::UnconfirmedTx
            );
            // if ConfirmedIndex not set, due to confirm height not beyond genesis height
            let confirmed = Self::confirmed_index().ok_or(Error::<T, I>::UnconfirmedTx)?;
            let height = header_info.height;
            if height > confirmed.height {
                error!(
                "[apply_push_transaction] Receive an unconfirmed tx (height:{}, hash:{:?}), confirmed index (height:{}, hash:{:?})", 
                height, tx_hash, confirmed.height, confirmed.hash
            );
                return Err(Error::<T, I>::UnconfirmedTx.into());
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
                        return Err(Error::<T, I>::ReplayedTx.into());
                    }
                }
            }

            let network = Pallet::<T, I>::network_id();
            let min_deposit = Pallet::<T, I>::btc_min_deposit();
            let current_trustee_pair = get_current_trustee_address_pair::<T, I>()?;
            let last_trustee_pair = get_last_trustee_address_pair::<T, I>().ok();
            let state = tx::process_tx::<T, I>(
                tx.raw,
                prev_tx,
                network,
                min_deposit,
                current_trustee_pair,
                last_trustee_pair,
            );
            TxState::<T, I>::insert(&tx_hash, state);
            Self::deposit_event(Event::<T, I>::TxProcessed(tx_hash, block_hash, state));
            match state.result {
                BtcTxResult::Success => Ok(()),
                BtcTxResult::Failure => Err(Error::<T, I>::ProcessTxFailed.into()),
            }
        }
    }
    impl<T: Config<I>, I: 'static> ChainT<BalanceOf<T>> for Pallet<T, I> {
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
                    try_addr(addr)
                );
                err
            })?;

            match get_current_trustee_address_pair::<T, I>() {
                Ok((hot_addr, cold_addr)) => {
                    // do not allow withdraw from trustee address
                    if address == hot_addr || address == cold_addr {
                        return Err(Error::<T, I>::InvalidAddress.into());
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
        ) -> Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
            if *asset_id != Self::ASSET_ID {
                return Err(xpallet_assets::Error::<T>::ActionNotAllowed.into());
            }
            let fee = Self::btc_withdrawal_fee().saturated_into();
            let limit = WithdrawalLimit::<BalanceOf<T>> {
                minimal_withdrawal: fee * 3u32.saturated_into() / 2u32.saturated_into(),
                fee,
            };
            Ok(limit)
        }
    }
}
