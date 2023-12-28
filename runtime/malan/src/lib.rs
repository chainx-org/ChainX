// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.
#![allow(clippy::unnecessary_cast)]
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use static_assertions::const_assert;

use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
#[cfg(feature = "runtime-benchmarks")]
use sp_runtime::RuntimeString;
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        self, AccountIdConversion, BlakeTwo256, Block as BlockT, Convert, DispatchInfoOf,
        NumberFor, OpaqueKeys, SaturatedConversion, Saturating, SignedExtension, StaticLookup,
    },
    transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
        TransactionValidityError, ValidTransaction,
    },
    ApplyExtrinsicResult, DispatchError, Perbill, Percent, Permill, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_system::EnsureRoot;
use pallet_grandpa::{
    fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots;

use chainx_runtime_common::{BlockLength, BlockWeights, BASE_FEE};
use xpallet_dex_spot::{Depth, FullPairInfo, RpcOrder, TradingPairId};
use xpallet_mining_asset::{MinerLedger, MiningAssetInfo, MiningDividendInfo};
use xpallet_mining_staking::{NominatorInfo, NominatorLedger, ValidatorInfo};
use xpallet_support::traits::MultisigAddressFor;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, debug, parameter_types,
    traits::{
        ConstBool, ConstU32, Contains, Currency, EnsureOneOf, EqualPrivilegeOnly, Get, Imbalance,
        InstanceFilter, KeyOwnerProofSystem, LockIdentifier, OnRuntimeUpgrade, OnUnbalanced,
        Randomness,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        Weight,
    },
    PalletId, StorageValue,
};
pub use pallet_timestamp::Call as TimestampCall;

pub use chainx_primitives::{
    AccountId, AccountIndex, AddrStr, Amount, AssetId, Balance, BlockNumber, ChainAddress, Hash,
    Index, Moment, ReferralId, Signature, Token,
};
pub use sp_staking::SessionIndex;
pub use xp_protocol::*;
pub use xp_runtime::Memo;

// xpallet re-exports
pub use xpallet_assets::{
    AssetInfo, AssetRestrictions, AssetType, Chain, TotalAssetInfo, WithdrawalLimit,
};
#[cfg(feature = "std")]
pub use xpallet_gateway_bitcoin::h256_rev;
pub use xpallet_gateway_bitcoin::{
    hash_rev, types::BtcHeaderInfo, BtcHeader, BtcNetwork, BtcParams, BtcTxVerifier,
    BtcWithdrawalProposal, Compact, H256,
};
pub use xpallet_gateway_common::{
    trustees,
    types::{
        GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, ScriptInfo, TrusteeInfoConfig,
    },
};
pub use xpallet_gateway_records::{Withdrawal, WithdrawalRecordId};
pub use xpallet_mining_asset::MiningWeight;
pub use xpallet_mining_staking::VoteWeight;

/// Constant values used within the runtime.
pub mod constants;
/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
mod migrations;

use self::constants::{currency::*, time::*};
use self::impls::{ChargeExtraFee, DealWithBTCFees, DealWithFees, SlowAdjustingFeeUpdate};

// EVM
use chainx_runtime_common::NORMAL_DISPATCH_RATIO;
use fp_rpc::TransactionStatus;
use pallet_ethereum::{Call::transact, Transaction as EthereumTransaction};
#[cfg(feature = "std")]
pub use pallet_evm::GenesisAccount;
use pallet_evm::{
    Account as EVMAccount, EnsureAddressNever, EnsureAddressRoot, FeeCalculator,
    HashedAddressMapping, Runner,
};
use sp_core::{H160, U256};
use sp_runtime::traits::{Dispatchable, PostDispatchInfoOf};
mod precompiles;
mod withdraw;

pub use precompiles::ChainXPrecompiles;

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("chainx"),
    impl_name: create_runtime_str!("chainx-malan"),
    authoring_version: 1,
    spec_version: 33,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 7,
    state_version: 0,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// The BABE epoch configuration at genesis.
/// The existing chain is running with PrimaryAndSecondaryPlainSlots,
/// you should keep returning the same thing in BabeApi::configuration()
/// as you were doing before.
///
/// Edit: it's okay to change this here as BabeApi::configuration()
/// is only used on genesis, so this change won't have any effect on
/// the existing chains. But maybe it makes it more clear if you still
/// keep the original value.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: PRIMARY_PROBABILITY,
        allowed_slots: PrimaryAndSecondaryPlainSlots,
    };

#[derive(Debug, Clone, Eq, PartialEq, codec::Encode, codec::Decode, MaxEncodedLen, TypeInfo)]
pub struct BaseFilter;
impl Contains<Call> for BaseFilter {
    fn contains(call: &Call) -> bool {
        use frame_support::dispatch::GetCallMetadata;

        let metadata = call.get_call_metadata();
        !XSystem::is_paused(metadata)
    }
}

pub const FORBIDDEN_CALL: u8 = 255;
pub const FORBIDDEN_ACCOUNT: u8 = 254;

impl SignedExtension for BaseFilter {
    const IDENTIFIER: &'static str = "BaseFilter";
    type AccountId = AccountId;
    type Call = Call;
    type AdditionalSigned = ();
    type Pre = ();
    fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
        Ok(())
    }

    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        info: &DispatchInfoOf<Self::Call>,
        len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        self.validate(who, call, info, len).map(|_| ())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        if !Self::contains(call) {
            return Err(InvalidTransaction::Custom(FORBIDDEN_CALL).into());
        }
        if XSystem::blacklist(who) {
            return Err(InvalidTransaction::Custom(FORBIDDEN_ACCOUNT).into());
        }
        Ok(ValidTransaction::default())
    }
}

const AVERAGE_ON_INITIALIZE_WEIGHT: Perbill = Perbill::from_percent(10);
parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    /// We allow for 2 seconds of compute with a 6 second average block time.
    pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    /// Assume 10% of weight for average on_initialize calls.
    pub MaximumExtrinsicWeight: Weight =
        AvailableBlockRatio::get().saturating_sub(AVERAGE_ON_INITIALIZE_WEIGHT)
        * MaximumBlockWeight::get();
    pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub const Version: RuntimeVersion = VERSION;
    pub const SS58Prefix: u8 = xp_protocol::MAINNET_ADDRESS_FORMAT_ID;
}

const_assert!(
    AvailableBlockRatio::get().deconstruct() >= AVERAGE_ON_INITIALIZE_WEIGHT.deconstruct()
);

impl frame_system::Config for Runtime {
    type BaseCallFilter = BaseFilter;
    type BlockWeights = BlockWeights;
    type BlockLength = BlockLength;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = Indices;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const IndexDeposit: Balance = 10 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
    type AccountIndex = AccountIndex;
    type Currency = Balances;
    type Deposit = IndexDeposit;
    type Event = Event;
    type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const MaxAuthorities: u32 = 10_000;
}
impl pallet_authority_discovery::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 0;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = ImOnline;
}

parameter_types! {
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_BLOCKS as u64;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

pub struct ReportLongevity;

impl Get<u64> for ReportLongevity {
    fn get() -> u64 {
        xpallet_mining_staking::BondingDuration::<Runtime>::get() as u64
            * xpallet_mining_staking::SessionsPerEra::<Runtime>::get() as u64
            * EpochDuration::get()
    }
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;

    type DisabledValidators = Session;

    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::IdentificationTuple;

    type KeyOwnerProofSystem = Historical;

    type HandleEquivocation =
        pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
}

impl pallet_grandpa::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;
    type KeyOwnerProofSystem = Historical;
    type HandleEquivocation = pallet_grandpa::EquivocationHandler<
        Self::KeyOwnerIdentification,
        Offences,
        ReportLongevity,
    >;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const Offset: BlockNumber = 0;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: Babe,
        pub grandpa: Grandpa,
        pub im_online: ImOnline,
        pub authority_discovery: AuthorityDiscovery,
    }
}

/// Substrate has the controller/stash concept, the according `Convert`
/// implementation is used to find the stash of the given controller
/// account. There is no such concept in the context of ChainX, the
/// _stash_ account is also the _controller_ account.
pub struct SimpleValidatorIdConverter;

impl Convert<AccountId, Option<AccountId>> for SimpleValidatorIdConverter {
    fn convert(controller: AccountId) -> Option<AccountId> {
        Some(controller)
    }
}

impl pallet_session::Config for Runtime {
    type Event = Event;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = SimpleValidatorIdConverter;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    // We do not make use of the historical feature of pallet-session, hereby use XStaking only.
    type SessionManager = XStaking;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    /// No dusty accounts in ChainX.
    pub const ExistentialDeposit: Balance = 0;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MILLICENTS; // 100 => 0.000001 pcx
    pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
    type TransactionByteFee = TransactionByteFee;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = self::constants::fee::WeightToFee;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

impl xpallet_transaction_fee::Config for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_BLOCKS;
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
    /// We prioritize im-online heartbeats over election solution submission.
    pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::MAX / 2;
    pub const MaxKeys: u32 = 10_000;
    pub const MaxPeerInHeartbeats: u32 = 10_000;
    pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = ImOnlineId;
    type Event = Event;
    type ValidatorSet = Self;
    type NextSessionRotation = Babe;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
    type MaxKeys = MaxKeys;
    type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
    type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
}

impl frame_support::traits::ValidatorSet<AccountId> for Runtime {
    type ValidatorId = AccountId;
    type ValidatorIdOf = SimpleValidatorIdConverter;

    fn session_index() -> SessionIndex {
        Session::current_index()
    }

    fn validators() -> Vec<Self::ValidatorId> {
        Session::validators()
    }
}

impl frame_support::traits::ValidatorSetWithIdentification<AccountId> for Runtime {
    type Identification = AccountId;
    type IdentificationOf = SimpleValidatorIdConverter;
}

/// Dummy implementation for the trait bound of pallet_im_online.
/// We actually make no use of the historical feature of pallet_session.
impl pallet_session_historical::Config for Runtime {
    type FullIdentification = AccountId;
    /// Substrate: given the stash account ID, find the active exposure of nominators on that account.
    /// ChainX: the full identity is always the validator account itself.
    type FullIdentificationOf = SimpleValidatorIdConverter;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        public: <Signature as traits::Verify>::Signer,
        account: AccountId,
        nonce: Index,
    ) -> Option<(
        Call,
        <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload,
    )> {
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let tip = 0;
        let extra: SignedExtra = (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
            BaseFilter,
            ChargeExtraFee,
        );
        let raw_payload = SignedPayload::new(call, extra)
            .map_err(|e| {
                frame_support::log::warn!("Unable to create signed payload: {:?}", e);
            })
            .ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let address = Indices::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature, extra)))
    }
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as traits::Verify>::Signer;
    type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = Call;
}

impl pallet_offences::Config for Runtime {
    type Event = Event;
    type IdentificationTuple = xpallet_mining_staking::IdentificationTuple<Runtime>;
    type OnOffenceHandler = XStaking;
}

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const LaunchPeriod: BlockNumber = HOURS;
    pub const VotingPeriod: BlockNumber = HOURS;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
    pub const InstantAllowed: bool = true;
    // 10 PCX
    pub const MinimumDeposit: Balance = 1000 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = HOURS;
    pub const CooloffPeriod: BlockNumber = HOURS;
    // One cent: $10,000 / MB
    pub const PreimageByteDeposit: Balance = CENTS;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
    type Proposal = Call;
    type Event = Event;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod;
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
    type InstantOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
        EnsureRoot<AccountId>,
    >;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cooloff period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type PreimageByteDeposit = PreimageByteDeposit;
    type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = MaxVotes;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = HOURS;
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // 10 PCX
    pub const CandidacyBond: Balance = 1000 * DOLLARS;
    // 1 storage item created, key size is 32 bytes, value size is 16+16.
    pub const VotingBondBase: Balance = deposit(1, 64);
    // additional data per vote is 32 bytes (account id).
    pub const VotingBondFactor: Balance = deposit(0, 32);
    pub const VotingBond: Balance = DOLLARS;
    pub const TermDuration: BlockNumber = DAYS;
    pub const DesiredMembers: u32 = 5;
    pub const DesiredRunnersUp: u32 = 3;
    pub const ElectionsPhragmenPalletId: LockIdentifier = *b"pcx/phre";
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
    type Event = Event;
    type PalletId = ElectionsPhragmenPalletId;
    type Currency = Balances;
    type ChangeMembers = Council;
    // NOTE: this implies that council's genesis members cannot be set directly and must come from
    // this module.
    type InitializeMembers = Council;
    type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = Treasury;
    type KickedMember = Treasury;
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TechnicalMotionDuration: BlockNumber = HOURS;
    pub const TechnicalMaxProposals: u32 = 100;
    pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
    type MaxMembers = TechnicalMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

type EnsureRootOrHalfCouncil = EnsureOneOf<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;
impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
    type Event = Event;
    type AddOrigin = EnsureRootOrHalfCouncil;
    type RemoveOrigin = EnsureRootOrHalfCouncil;
    type SwapOrigin = EnsureRootOrHalfCouncil;
    type ResetOrigin = EnsureRootOrHalfCouncil;
    type PrimeOrigin = EnsureRootOrHalfCouncil;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    // 10 PCX
    pub const ProposalBondMinimum: Balance = 1000 * DOLLARS;
    // 100 PCX
    pub const ProposalBondMaximum: Balance = 10000 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 6 * DAYS;
    pub const NoBurn: Permill = Permill::from_percent(0);
    pub const TipCountdown: BlockNumber = DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = DOLLARS;
    pub const DataDepositPerByte: Balance = CENTS;
    pub const BountyDepositBase: Balance = DOLLARS;
    pub const BountyDepositPayoutDelay: BlockNumber = 4 * DAYS;
    pub const TreasuryPalletId: PalletId = PalletId(*b"pcx/trsy");
    pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
    pub const MaximumReasonLength: u32 = 16384;
    pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
    pub const BountyValueMinimum: Balance = 10 * DOLLARS;
    pub const MaxApprovals: u32 = 100;
}

impl pallet_treasury::Config for Runtime {
    type Currency = Balances;
    type ApproveOrigin = EnsureOneOf<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
    >;
    type RejectOrigin = EnsureOneOf<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
    >;
    type Event = Event;
    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ProposalBondMaximum;
    type SpendPeriod = SpendPeriod;
    type Burn = NoBurn;
    type PalletId = TreasuryPalletId;
    type BurnDestination = ();
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type SpendFunds = Bounties;
    type MaxApprovals = MaxApprovals;
}

impl pallet_bounties::Config for Runtime {
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type BountyCuratorDeposit = BountyCuratorDeposit;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type Event = Event;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
    type ChildBountyManager = ();
}

impl pallet_tips::Config for Runtime {
    type Event = Event;
    type MaximumReasonLength = MaximumReasonLength;
    type DataDepositPerByte = DataDepositPerByte;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type Tippers = Elections;
    type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * MaximumBlockWeight::get();
    // Retry a scheduled item every 10 blocks (1 minute) until the preimage exists.
    pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type MaxScheduledPerBlock = ConstU32<50>;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
    type PreimageProvider = ();
    type NoPreimagePostponement = NoPreimagePostponement;
}

parameter_types! {
    pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
    pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
    pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = Treasury;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type RegistrarOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // One storage item; key size 32, value size 8; .
    pub const ProxyDepositBase: Balance = deposit(1, 8);
    // Additional storage item size of 33 bytes.
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const MaxProxies: u16 = 32;
    pub const AnnouncementDepositBase: Balance = deposit(1, 8);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
    pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    RuntimeDebug,
    MaxEncodedLen,
    TypeInfo,
)]
pub enum ProxyType {
    Any = 0,
    NonTransfer = 1,
    Governance = 2,
    Staking = 3,
    IdentityJudgement = 4,
    CancelProxy = 5,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}

impl InstanceFilter<Call> for ProxyType {
    fn filter(&self, c: &Call) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => matches!(
                c,
                Call::System(..)
                    | Call::Scheduler(..)
                    | Call::Babe(..)
                    | Call::Timestamp(..)
                    | Call::Indices(pallet_indices::Call::claim{..})
                    | Call::Indices(pallet_indices::Call::free{..})
                    | Call::Indices(pallet_indices::Call::freeze{..})
                    // Specifically omitting Indices `transfer`, `force_transfer`
                    // Specifically omitting the entire Balances pallet
                    | Call::Authorship(..)
                    | Call::XStaking(..)
                    | Call::Session(..)
                    | Call::Grandpa(..)
                    | Call::ImOnline(..)
                    | Call::Democracy(..)
                    | Call::Council(..)
                    | Call::TechnicalCommittee(..)
                    | Call::Elections(..)
                    | Call::TechnicalMembership(..)
                    | Call::Treasury(..)
                    | Call::Utility(..)
                    | Call::Identity(..)
                    | Call::Proxy(..)
                    | Call::Multisig(..)
            ),
            ProxyType::Governance => matches!(
                c,
                Call::Democracy(..)
                    | Call::Council(..)
                    | Call::TechnicalCommittee(..)
                    | Call::Elections(..)
                    | Call::Treasury(..)
                    | Call::Utility(..)
            ),
            ProxyType::Staking => matches!(
                c,
                Call::XStaking(..) | Call::Session(..) | Call::Utility(..)
            ),
            ProxyType::IdentityJudgement => matches!(
                c,
                Call::Identity(pallet_identity::Call::provide_judgement { .. }) | Call::Utility(..)
            ),
            ProxyType::CancelProxy => {
                matches!(
                    c,
                    Call::Proxy(pallet_proxy::Call::reject_announcement { .. })
                )
            }
        }
    }
    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (x, y) if x == y => true,
            (ProxyType::Any, _) => true,
            (_, ProxyType::Any) => false,
            (ProxyType::NonTransfer, _) => true,
            _ => false,
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = MaxProxies;
    type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
    type MaxPending = MaxPending;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

///////////////////////////////////////////
// Chainx pallets
///////////////////////////////////////////
impl xpallet_system::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
}

parameter_types! {
    pub const ChainXAssetId: AssetId = xp_protocol::PCX;
}

impl xpallet_assets_registrar::Config for Runtime {
    type Event = Event;
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = XMiningAsset;
    type WeightInfo = xpallet_assets_registrar::weights::SubstrateWeight<Runtime>;
}

impl xpallet_assets::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type TreasuryAccount = SimpleTreasuryAccount;
    type OnCreatedAccount = frame_system::Provider<Runtime>;
    type OnAssetChanged = XMiningAsset;
    type WeightInfo = xpallet_assets::weights::SubstrateWeight<Runtime>;
}

impl xpallet_gateway_records::Config for Runtime {
    type Event = Event;
    type WeightInfo = xpallet_gateway_records::weights::SubstrateWeight<Runtime>;
}

pub struct MultisigProvider;
impl MultisigAddressFor<AccountId> for MultisigProvider {
    fn calc_multisig(who: &[AccountId], threshold: u16) -> AccountId {
        Multisig::multi_account_id(who, threshold)
    }
}

impl xpallet_gateway_common::Config for Runtime {
    type Event = Event;
    type Validator = XStaking;
    type DetermineMultisigAddress = MultisigProvider;
    type CouncilOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
    type Bitcoin = XGatewayBitcoin;
    type BitcoinTrustee = XGatewayBitcoin;
    type BitcoinTrusteeSessionProvider = trustees::bitcoin::BtcTrusteeSessionManager<Runtime>;
    type BitcoinTotalSupply = XGatewayBitcoin;
    type BitcoinWithdrawalProposal = XGatewayBitcoin;
    type WeightInfo = xpallet_gateway_common::weights::SubstrateWeight<Runtime>;
}

impl xpallet_gateway_bitcoin::Config for Runtime {
    type Event = Event;
    type UnixTime = Timestamp;
    type CouncilOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
    type AccountExtractor = xp_gateway_bitcoin::OpReturnExtractor;
    type TrusteeSessionProvider = trustees::bitcoin::BtcTrusteeSessionManager<Runtime>;
    type TrusteeInfoUpdate = XGatewayCommon;
    type ReferralBinding = XGatewayCommon;
    type AddressBinding = XGatewayCommon;
    type WeightInfo = xpallet_gateway_bitcoin::weights::SubstrateWeight<Runtime>;
}

impl xpallet_dex_spot::Config for Runtime {
    type Event = Event;
    type Price = Balance;
    type WeightInfo = xpallet_dex_spot::weights::SubstrateWeight<Runtime>;
}

pub struct SimpleTreasuryAccount;
impl xpallet_support::traits::TreasuryAccount<AccountId> for SimpleTreasuryAccount {
    fn treasury_account() -> Option<AccountId> {
        Some(TreasuryPalletId::get().into_account())
    }
}

parameter_types! {
    // Total issuance is 7723350PCX by the end of ChainX 1.0.
    // 210000 - (7723350 / 50) = 55533
    pub const MigrationSessionOffset: SessionIndex = 55533;
    pub const MinimumReferralId: u32 = 2;
    pub const MaximumReferralId: u32 = 12;
}

impl xpallet_mining_staking::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type SessionDuration = SessionDuration;
    type MinimumReferralId = MinimumReferralId;
    type MaximumReferralId = MaximumReferralId;
    type SessionInterface = Self;
    type TreasuryAccount = SimpleTreasuryAccount;
    type AssetMining = XMiningAsset;
    type DetermineRewardPotAccount =
        xpallet_mining_staking::SimpleValidatorRewardPotAccountDeterminer<Runtime>;
    type ValidatorRegistration = Session;
    type WeightInfo = xpallet_mining_staking::weights::SubstrateWeight<Runtime>;
}

pub struct ReferralGetter;
impl xpallet_mining_asset::GatewayInterface<AccountId> for ReferralGetter {
    fn referral_of(who: &AccountId, asset_id: AssetId) -> Option<AccountId> {
        use xpallet_gateway_common::traits::ReferralBinding;
        XGatewayCommon::referral(&asset_id, who)
    }
}

impl xpallet_mining_asset::Config for Runtime {
    type Event = Event;
    type StakingInterface = Self;
    type GatewayInterface = ReferralGetter;
    type TreasuryAccount = SimpleTreasuryAccount;
    type DetermineRewardPotAccount =
        xpallet_mining_asset::SimpleAssetRewardPotAccountDeterminer<Runtime>;
    type WeightInfo = xpallet_mining_asset::weights::SubstrateWeight<Runtime>;
}

impl xpallet_genesis_builder::Config for Runtime {}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

impl xpallet_ethereum_chain_id::Config for Runtime {}

impl xpallet_btc_ledger::Config for Runtime {
    type Balance = Balance;
    type Event = Event;
    type CouncilOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
    type PalletId = TreasuryPalletId;
}

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = 60_000;

/// Maximum weight per block
pub const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;


parameter_types! {
    // 2_500_000
    pub BlockGasLimit: U256
        = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT / WEIGHT_PER_GAS);
    pub PrecompilesValue: ChainXPrecompiles<Runtime> = ChainXPrecompiles::<_>::new();
}

pub struct ChainXGasWeightMapping;
impl pallet_evm::GasWeightMapping for ChainXGasWeightMapping {
    fn gas_to_weight(gas: u64) -> Weight {
        gas.saturating_mul(WEIGHT_PER_GAS)
    }
    fn weight_to_gas(weight: Weight) -> u64 {
        weight.wrapping_div(WEIGHT_PER_GAS)
    }
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = BaseFee;
    type GasWeightMapping = ChainXGasWeightMapping;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = HashedAddressMapping<BlakeTwo256>;
    type Currency = XBtcLedger;
    type Event = Event;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = ChainXPrecompiles<Runtime>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = EthereumChainId;
    type OnChargeTransaction = pallet_evm::EVMCurrencyAdapter<XBtcLedger, DealWithBTCFees>;
    type BlockGasLimit = BlockGasLimit;
    type FindAuthor = ();
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

impl pallet_ethereum::Config for Runtime {
    type Event = Event;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
}

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(BASE_FEE);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> Permill {
        Permill::zero()
    }
    fn ideal() -> Permill {
        Permill::from_parts(500_000)
    }
    fn upper() -> Permill {
        Permill::from_parts(1_000_000)
    }
}

impl pallet_base_fee::Config for Runtime {
    type Event = Event;
    type Threshold = BaseFeeThreshold;
    // Tells `pallet_base_fee` whether to calculate a new BaseFee `on_finalize` or not.
    type IsActive = ConstBool<false>;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
}

parameter_types! {
    // 0x1111111111111111111111111111111111111111
    pub EvmCaller: H160 = H160::from_slice(&[17u8;20][..]);
    pub ClaimBond: Balance = PCXS;
}
impl xpallet_assets_bridge::Config for Runtime {
    type Event = Event;
    type EvmCaller = EvmCaller;
    type ClaimBond = ClaimBond;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = chainx_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Basic stuff.
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 2,

        // Must be before session.
        Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned} = 3,

        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 4,
        Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 6,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 7,

        // Consensus support.
        Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent} = 8,
        Offences: pallet_offences::{Pallet, Storage, Event} = 9,
        Historical: pallet_session_historical::{Pallet} = 10,
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 11,
        Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 12,
        ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 13,
        AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config} = 14,

        // Governance stuff.
        Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 15,
        Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 16,
        TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 17,
        Elections: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>} = 18,
        TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 19,
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 20,

        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 21,

        Utility: pallet_utility::{Pallet, Call, Event} = 22,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 23,

        // ChainX basics.
        XSystem: xpallet_system::{Pallet, Call, Storage, Event<T>, Config} = 24,
        XAssetsRegistrar: xpallet_assets_registrar::{Pallet, Call, Storage, Event<T>, Config} = 25,
        XAssets: xpallet_assets::{Pallet, Call, Storage, Event<T>, Config<T>} = 26,

        // Mining, must be after XAssets.
        XStaking: xpallet_mining_staking::{Pallet, Call, Storage, Event<T>, Config<T>} = 27,
        XMiningAsset: xpallet_mining_asset::{Pallet, Call, Storage, Event<T>, Config<T>} = 28,

        // Crypto gateway stuff.
        XGatewayRecords: xpallet_gateway_records::{Pallet, Call, Storage, Event<T>} = 29,
        XGatewayCommon: xpallet_gateway_common::{Pallet, Call, Storage, Event<T>, Config<T>} = 30,
        XGatewayBitcoin: xpallet_gateway_bitcoin::{Pallet, Call, Storage, Event<T>, Config<T>} = 31,

        // DEX
        XSpot: xpallet_dex_spot::{Pallet, Call, Storage, Event<T>, Config<T>} = 32,

        XGenesisBuilder: xpallet_genesis_builder::{Pallet, Config<T>} = 33,

        // It might be possible to merge this module into pallet_transaction_payment in future, thus
        // we put it at the end for keeping the extrinsic ordering.
        XTransactionFee: xpallet_transaction_fee::{Pallet, Event<T>} = 35,

        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 36,

        Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 37,
        Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 38,

        // Put Sudo last so that the extrinsic ordering stays the same once it's removed.
        Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 39,

        // Ethereum compatibility
        EthereumChainId: xpallet_ethereum_chain_id::{Pallet, Call, Storage, Config} = 40,
        Evm: pallet_evm::{Pallet, Config, Call, Storage, Event<T>} = 41,
        Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config, Origin} = 42,
        BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 44,

        // Dependency on xpallet_assets and pallet_evm
        XAssetsBridge: xpallet_assets_bridge::{Pallet, Call, Storage, Config<T>, Event<T>} = 45,

        XBtcLedger: xpallet_btc_ledger::{Pallet, Call, Storage, Config<T>, Event<T>} = 46,
    }
);

/// The address format for describing accounts.
pub type Address = <Indices as StaticLookup>::Source;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    BaseFilter,
    ChargeExtraFee,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = fp_self_contained::CheckedExtrinsic<AccountId, Call, SignedExtra, H160>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

pub struct TransactionConverter;
impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
        UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        )
    }
}
impl fp_rpc::ConvertTransaction<sp_runtime::OpaqueExtrinsic> for TransactionConverter {
    fn convert_transaction(
        &self,
        transaction: pallet_ethereum::Transaction,
    ) -> sp_runtime::OpaqueExtrinsic {
        let extrinsic = UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        );
        let encoded = extrinsic.encode();
        sp_runtime::OpaqueExtrinsic::decode(&mut &encoded[..])
            .expect("Encoded extrinsic is always valid")
    }
}

impl fp_self_contained::SelfContainedCall for Call {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            Call::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            Call::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<Call>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            Call::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<Call>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            Call::Ethereum(call) => call.pre_dispatch_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ Call::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(call.dispatch(
                Origin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info)),
            )),
            _ => None,
        }
    }
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            sp_consensus_babe::BabeGenesisConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: BABE_GENESIS_EPOCH_CONFIG.c,
                genesis_authorities: Babe::authorities().to_vec(),
                randomness: Babe::randomness(),
                allowed_slots: BABE_GENESIS_EPOCH_CONFIG.allowed_slots,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::Slot {
            Babe::current_epoch_start()
        }

        fn current_epoch() -> sp_consensus_babe::Epoch {
            Babe::current_epoch()
        }

        fn next_epoch() -> sp_consensus_babe::Epoch {
            Babe::next_epoch()
        }

        fn generate_key_ownership_proof(
            _slot: sp_consensus_babe::Slot,
            authority_id: sp_consensus_babe::AuthorityId,
        ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
            Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
            key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Babe::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> fg_primitives::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Grandpa::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            Historical::prove((fg_primitives::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(fg_primitives::OpaqueKeyOwnershipProof::new)
        }
    }

    impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
        fn authorities() -> Vec<AuthorityDiscoveryId> {
            AuthorityDiscovery::authorities()
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            if let Some(extra_fee) = ChargeExtraFee::has_extra_fee(&uxt.0.function) {
                let base_info = TransactionPayment::query_info(uxt, len);
                pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo {
                    partial_fee: base_info.partial_fee + extra_fee,
                    ..base_info
                }
            } else {
                TransactionPayment::query_info(uxt, len)
            }
        }
        fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }

    impl xpallet_transaction_fee_rpc_runtime_api::XTransactionFeeApi<Block, Balance> for Runtime {
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> xpallet_transaction_fee::FeeDetails<Balance> {
            let maybe_extra = ChargeExtraFee::has_extra_fee(&uxt.0.function);
            let base = TransactionPayment::query_fee_details(uxt, len);
            xpallet_transaction_fee::FeeDetails::new(base, maybe_extra)
        }
    }

    impl xpallet_assets_rpc_runtime_api::XAssetsApi<Block, AccountId, Balance> for Runtime {
        fn assets_for_account(who: AccountId) -> BTreeMap<AssetId, BTreeMap<AssetType, Balance>> {
            XAssets::valid_assets_of(&who)
        }

        fn assets() -> BTreeMap<AssetId, TotalAssetInfo<Balance>> {
            XAssets::total_asset_infos()
        }
    }

    impl xpallet_mining_staking_rpc_runtime_api::XStakingApi<Block, AccountId, Balance, VoteWeight, BlockNumber> for Runtime {
        fn validators() -> Vec<ValidatorInfo<AccountId, Balance, VoteWeight, BlockNumber>> {
            XStaking::validators_info()
        }
        fn validator_info_of(who: AccountId) -> ValidatorInfo<AccountId, Balance, VoteWeight, BlockNumber> {
            XStaking::validator_info_of(who)
        }
        fn staking_dividend_of(who: AccountId) -> BTreeMap<AccountId, Balance> {
            XStaking::staking_dividend_of(who)
        }
        fn nomination_details_of(who: AccountId) -> BTreeMap<AccountId, NominatorLedger<Balance, VoteWeight, BlockNumber>> {
            XStaking::nomination_details_of(who)
        }
        fn nominator_info_of(who: AccountId) -> NominatorInfo<BlockNumber> {
            XStaking::nominator_info_of(who)
        }
    }

    impl xpallet_dex_spot_rpc_runtime_api::XSpotApi<Block, AccountId, Balance, BlockNumber, Balance> for Runtime {
        fn trading_pairs() -> Vec<FullPairInfo<Balance, BlockNumber>> {
            XSpot::trading_pairs()
        }

        fn orders(who: AccountId, page_index: u32, page_size: u32) -> Vec<RpcOrder<TradingPairId, AccountId, Balance, Balance, BlockNumber>> {
            XSpot::orders(who, page_index, page_size)
        }

        fn depth(pair_id: TradingPairId, depth_size: u32) -> Option<Depth<Balance, Balance>> {
            XSpot::depth(pair_id, depth_size)
        }
    }

    impl xpallet_mining_asset_rpc_runtime_api::XMiningAssetApi<Block, AccountId, Balance, MiningWeight, BlockNumber> for Runtime {
        fn mining_assets() -> Vec<MiningAssetInfo<AccountId, Balance, MiningWeight, BlockNumber>> {
            XMiningAsset::mining_assets()
        }

        fn mining_dividend(who: AccountId) -> BTreeMap<AssetId, MiningDividendInfo<Balance>> {
            XMiningAsset::mining_dividend(who)
        }

        fn miner_ledger(who: AccountId) -> BTreeMap<AssetId, MinerLedger<MiningWeight, BlockNumber>> {
            XMiningAsset::miner_ledger(who)
        }
    }

    impl xpallet_gateway_records_rpc_runtime_api::XGatewayRecordsApi<Block, AccountId, Balance, BlockNumber> for Runtime {
        fn withdrawal_list() -> BTreeMap<u32, Withdrawal<AccountId, Balance, BlockNumber>> {
            XGatewayRecords::withdrawal_list()
        }

        fn withdrawal_list_by_chain(chain: Chain) -> BTreeMap<u32, Withdrawal<AccountId, Balance, BlockNumber>> {
            XGatewayRecords::withdrawals_list_by_chain(chain)
        }
    }

    impl xpallet_gateway_bitcoin_rpc_runtime_api::XGatewayBitcoinApi<Block, AccountId> for Runtime {
        fn verify_tx_valid(
            raw_tx: Vec<u8>,
            withdrawal_id_list: Vec<u32>,
            full_amount: bool,
        ) -> Result<bool, DispatchError> {
            XGatewayBitcoin::verify_tx_valid(raw_tx, withdrawal_id_list, full_amount)
        }

        fn get_withdrawal_proposal() -> Option<BtcWithdrawalProposal<AccountId>> {
            XGatewayBitcoin::get_withdrawal_proposal()
        }

        fn get_genesis_info() -> (BtcHeader, u32) {
            XGatewayBitcoin::get_genesis_info()
        }

        fn get_btc_block_header(txid: H256) -> Option<BtcHeaderInfo> {
            XGatewayBitcoin::get_btc_block_header(txid)
        }
    }

    impl xpallet_btc_ledger_runtime_api::BtcLedgerApi<Block, AccountId, Balance> for Runtime {
        fn get_balance(who: AccountId) -> Balance {
            XBtcLedger::free_balance(&who)
        }
        fn get_total() -> Balance {
            XBtcLedger::get_total()
        }
    }

    impl xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<Block, AccountId, Balance, BlockNumber> for Runtime {
        fn bound_addrs(who: AccountId) -> BTreeMap<Chain, Vec<ChainAddress>> {
            XGatewayCommon::bound_addrs(&who)
        }

        fn withdrawal_limit(asset_id: AssetId) -> Result<WithdrawalLimit<Balance>, DispatchError> {
            XGatewayCommon::withdrawal_limit(&asset_id)
        }

        #[allow(clippy::type_complexity)]
        fn withdrawal_list_with_fee_info(asset_id: AssetId) -> Result<
            BTreeMap<
                WithdrawalRecordId,
                (
                    Withdrawal<AccountId, Balance, BlockNumber>,
                    WithdrawalLimit<Balance>,
                ),
            >,
            DispatchError,
        >
        {
            XGatewayCommon::withdrawal_list_with_fee_info(&asset_id)
        }

        fn verify_withdrawal(asset_id: AssetId, value: Balance, addr: AddrStr, memo: Memo) -> Result<(), DispatchError> {
            XGatewayCommon::verify_withdrawal(asset_id, value, &addr, &memo)
        }

        fn trustee_multisigs() -> BTreeMap<Chain, AccountId> {
            XGatewayCommon::trustee_multisigs()
        }

        fn trustee_properties(chain: Chain, who: AccountId) -> Option<GenericTrusteeIntentionProps<AccountId>> {
            XGatewayCommon::trustee_intention_props_of(who, chain)
        }

        fn trustee_session_info(chain: Chain, session_number: i32) -> Option<GenericTrusteeSessionInfo<AccountId, BlockNumber>> {
            if session_number < 0 {
                let number = match session_number {
                    -1i32 => Some(XGatewayCommon::trustee_session_info_len(chain)),
                    -2i32 => XGatewayCommon::trustee_session_info_len(chain).checked_sub(1),
                    _ => None
                };
                if let Some(number) = number {
                    XGatewayCommon::trustee_session_info_of(chain, number)
                }else{
                    None
                }
            }else{
                let number = session_number as u32;
                XGatewayCommon::trustee_session_info_of(chain, number)
            }

        }

        fn generate_trustee_session_info(chain: Chain, candidates: Vec<AccountId>) -> Result<(GenericTrusteeSessionInfo<AccountId, BlockNumber>, ScriptInfo<AccountId>), DispatchError> {
            let info = XGatewayCommon::try_generate_session_info(chain, candidates)?;
            // check multisig address
            let _ = XGatewayCommon::generate_multisig_addr(chain, &info.0)?;
            Ok(info)
        }
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
            UncheckedExtrinsic::new_unsigned(
                pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
            )
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            <Runtime as pallet_evm::Config>::ChainId::get()
        }

        fn account_basic(address: H160) -> EVMAccount {
            Evm::account_basic(&address)
        }

        fn gas_price() -> U256 {
            <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price()
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            Evm::account_codes(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            Evm::account_storages(address, H256::from_slice(&tmp[..]))
        }

        #[allow(clippy::redundant_closure)]
        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let is_transactional = false;
            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.low_u64(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                config.as_ref().unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.into())
        }

        #[allow(clippy::redundant_closure)]
        fn create(
            from: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let is_transactional = false;
            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit.low_u64(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                config.as_ref().unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.into())
        }

        fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
            Ethereum::current_transaction_statuses()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            Ethereum::current_block()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            Ethereum::current_receipts()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<TransactionStatus>>
        ) {
            (
                Ethereum::current_block(),
                Ethereum::current_receipts(),
                Ethereum::current_transaction_statuses()
            )
        }

        fn extrinsic_filter(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> Vec<EthereumTransaction> {
            xts.into_iter().filter_map(|xt| match xt.0.function {
                Call::Ethereum(transact { transaction }) => Some(transaction),
                _ => None
            }).collect::<Vec<EthereumTransaction>>()
        }

        fn elasticity() -> Option<Permill> {
            Some(BaseFee::elasticity())
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade() -> (Weight, Weight) {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here. If any of the pre/post migration checks fail, we shall stop
            // right here and right now.
            let weight = Executive::try_runtime_upgrade().unwrap();
            (weight, BlockWeights::get().max_block)
        }

        fn execute_block_no_check(block: Block) -> Weight {
            Executive::execute_block_no_check(block)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;

            let mut list = Vec::<BenchmarkList>::new();

            list_benchmark!(list, extra, xpallet_assets, XAssets);
            list_benchmark!(list, extra, xpallet_assets_registrar, XAssetsRegistrar);
            list_benchmark!(list, extra, xpallet_mining_asset, XMiningAsset);
            list_benchmark!(list, extra, xpallet_mining_staking, XStaking);
            list_benchmark!(list, extra, xpallet_gateway_records, XGatewayRecords);
            list_benchmark!(list, extra, xpallet_gateway_common, XGatewayCommon);
            list_benchmark!(list, extra, xpallet_gateway_bitcoin, XGatewayBitcoin);
            list_benchmark!(list, extra, xpallet_dex_spot, XSpot);

            let storage_info = AllPalletsWithSystem::storage_info();

            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};

            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
                // // Treasury Account
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95ecffd7b6c0f78751baa9d281e0bfa3a6d6f646c70792f74727372790000000000000000000000000000000000000000").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmarks!(params, batches);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    define_benchmarks!(
        [frame_benchmarking, BaselineBench::<Runtime>]
        [frame_system, SystemBench::<Runtime>]
        [xpallet_assets, XAssets]
        [xpallet_assets_registrar, XAssetsRegistrar]
        [xpallet_mining_asset, XMiningAsset]
        [xpallet_mining_staking, XStaking]
        [xpallet_gateway_records, XGatewayRecords]
        [xpallet_gateway_common,  XGatewayCommon]
        [xpallet_gateway_bitcoin, XGatewayBitcoin]
        [xpallet_dex_spot, XSpot]
    );
}
