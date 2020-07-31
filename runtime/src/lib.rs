//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use static_assertions::const_assert;

use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
    crypto::KeyTypeId,
    u32_trait::{_1, _2, _3, _4},
    OpaqueMetadata,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, BlakeTwo256, Block as BlockT, Convert, DispatchInfoOf, IdentityLookup,
        NumberFor, OpaqueKeys, Saturating, SignedExtension,
    },
    transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
        TransactionValidityError, ValidTransaction,
    },
    ApplyExtrinsicResult, DispatchError, FixedPointNumber, ModuleId, Perbill, Percent, Permill,
    Perquintill,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_system::{EnsureOneOf, EnsureRoot};
use pallet_grandpa::fg_primitives;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

use xpallet_mining_staking::{RpcNominatorLedger, ValidatorInfo};
use xpallet_support::RpcBalance;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{
        Currency, Filter, InstanceFilter, KeyOwnerProofSystem, LockIdentifier, OnUnbalanced,
        Randomness,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        IdentityFee, Weight,
    },
    StorageValue,
};
pub use pallet_timestamp::Call as TimestampCall;

pub use chainx_primitives::{
    AccountId, AccountIndex, AddrStr, AssetId, Balance, BlockNumber, Hash, Index, Memo, Moment,
    Name, Signature, Token,
};
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use xpallet_contracts_rpc_runtime_api::ContractExecResult;

// xpallet re-exports
pub use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
pub use xpallet_assets::{
    AssetInfo, AssetRestriction, AssetRestrictions, AssetType, Chain, TotalAssetInfo,
    WithdrawalLimit,
};
pub use xpallet_contracts::Schedule as ContractsSchedule;
pub use xpallet_contracts_primitives::XRC20Selector;
#[cfg(feature = "std")]
pub use xpallet_gateway_bitcoin::h256_conv_endian_from_str;
pub use xpallet_gateway_bitcoin::{
    BtcHeader, BtcNetwork, BtcParams, BtcTxVerifier, Compact as BtcCompact, H256 as BtcHash,
};
pub use xpallet_gateway_common::{
    trustees,
    types::{GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, TrusteeInfoConfig},
};
pub use xpallet_gateway_records::Withdrawal;
pub use xpallet_protocol::*;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
use impls::{Author, CurrencyToVoteHandler};

/// Constant values used within the runtime.
pub mod constants;
pub use constants::{currency::*, time::*};

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
        pub im_online: ImOnline,
    }
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("chainx"),
    impl_name: create_runtime_str!("chainx-net"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

#[derive(Debug, Clone, Eq, PartialEq, codec::Encode, codec::Decode)]
pub struct BaseFilter;
impl Filter<Call> for BaseFilter {
    fn filter(call: &Call) -> bool {
        use frame_support::dispatch::GetCallMetadata;
        let metadata = call.get_call_metadata();
        !XSystem::is_paused(metadata)
    }
}
pub struct IsCallable;
frame_support::impl_filter_stack!(IsCallable, BaseFilter, Call, is_callable);

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

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        if !Self::filter(&call) {
            return Err(TransactionValidityError::from(InvalidTransaction::Custom(
                FORBIDDEN_CALL,
            )));
        }

        if XSystem::blocked_accounts(who).is_some() {
            return Err(TransactionValidityError::from(InvalidTransaction::Custom(
                FORBIDDEN_ACCOUNT,
            )));
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
}

const_assert!(
    AvailableBlockRatio::get().deconstruct() >= AVERAGE_ON_INITIALIZE_WEIGHT.deconstruct()
);

impl frame_system::Trait for Runtime {
    type BaseCallFilter = BaseFilter;
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
    type Lookup = IdentityLookup<AccountId>;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Maximum weight of each block.
    type MaximumBlockWeight = MaximumBlockWeight;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// The weight of the overhead invoked on the block import process, independent of the
    /// extrinsics included in that block.
    type BlockExecutionWeight = BlockExecutionWeight;
    /// The base weight of any extrinsic processed by the runtime, independent of the
    /// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
    type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
    /// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
    /// idependent of the logic of that extrinsics. (Roughly max block weight - average on
    /// initialize cost).
    type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
    /// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
    type MaximumBlockLength = MaximumBlockLength;
    /// Portion of the block weight that is available to all normal transactions.
    type AvailableBlockRatio = AvailableBlockRatio;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type ModuleToIndex = ModuleToIndex;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Trait for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (ImOnline);
}

impl pallet_aura::Trait for Runtime {
    type AuthorityId = AuraId;
}

impl pallet_grandpa::Trait for Runtime {
    type Event = Event;
    type Call = Call;

    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;

    type KeyOwnerProofSystem = ();

    type HandleEquivocation = ();
}

parameter_types! {
    pub const Offset: BlockNumber = 0;
    pub const Period: BlockNumber = 50;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

/// Substrate has the controller/stash concept, the according `Convert` implementation
/// is used to find the stash of the given controller account.
/// There is no such concepts in the context of ChainX, the stash account is also the controller account.
pub struct SimpleValidatorIdConverter;

impl Convert<AccountId, Option<AccountId>> for SimpleValidatorIdConverter {
    fn convert(controller: AccountId) -> Option<AccountId> {
        Some(controller)
    }
}

impl pallet_session::Trait for Runtime {
    type Event = Event;
    type ValidatorId = <Self as frame_system::Trait>::AccountId;
    type ValidatorIdOf = SimpleValidatorIdConverter;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = XStaking;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * DOLLARS;
}

impl pallet_balances::Trait for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Runtime>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1; // TODO change in future
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}
type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(fees) = fees_then_tips.next() {
            // for fees, 80% to treasury, 20% to author
            // let mut split = fees.ration(80, 20);
            // if let Some(tips) = fees_then_tips.next() {
            //     // for tips, if any, 80% to treasury, 20% to author (though this can be anything)
            //     tips.ration_merge_into(80, 20, &mut split);
            // }
            // Treasury::on_unbalanced(split.0);
            // Author::on_unbalanced(split.1);
            // TODO impl fees dispatch
        }
    }
}

impl pallet_transaction_payment::Trait for Runtime {
    type Currency = Balances;
    type OnTransactionPayment = DealWithFees;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

parameter_types! {
    /// Babe use EPOCH_DURATION_IN_SLOTS here, we use Aura.
    pub const SessionDuration: BlockNumber = Period::get();
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
    /// We prioritize im-online heartbeats over election solution submission.
    pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
}

impl pallet_im_online::Trait for Runtime {
    type AuthorityId = ImOnlineId;
    type Event = Event;
    type SessionDuration = SessionDuration;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
}

/// Dummy implementation for the trait bound of pallet_im_online.
/// We actually make no use of the historical feature of pallet_session.
impl pallet_session::historical::Trait for Runtime {
    type FullIdentification = AccountId;
    /// Substrate: given the stash account ID, find the active exposure of nominators on that account.
    /// ChainX: we don't need such info due to the reward pot.
    type FullIdentificationOf = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = Call;
}

parameter_types! {
    pub OffencesWeightSoftLimit: Weight = Perbill::from_percent(60) * MaximumBlockWeight::get();
}

impl pallet_offences::Trait for Runtime {
    type Event = Event;
    type IdentificationTuple = xpallet_mining_staking::IdentificationTuple<Runtime>;
    type OnOffenceHandler = XStaking;
    type WeightSoftLimit = OffencesWeightSoftLimit;
}

impl pallet_utility::Trait for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Trait for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
}

impl pallet_sudo::Trait for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    pub const LaunchPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
    pub const VotingPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
    pub const InstantAllowed: bool = true;
    pub const MinimumDeposit: Balance = 100 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 30 * 24 * 60 * MINUTES;
    pub const CooloffPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
    // One cent: $10,000 / MB
    pub const PreimageByteDeposit: Balance = 1 * CENTS;
    pub const MaxVotes: u32 = 100;
}

impl pallet_democracy::Trait for Runtime {
    type Proposal = Call;
    type Event = Event;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin =
        pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin =
        pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin =
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin =
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, TechnicalCollective>;
    type InstantOrigin =
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin =
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cooloff period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type PreimageByteDeposit = PreimageByteDeposit;
    type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type MaxVotes = MaxVotes;
}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
    pub const CouncilMaxProposals: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Trait<CouncilCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
}

parameter_types! {
    pub const CandidacyBond: Balance = 10 * DOLLARS;
    pub const VotingBond: Balance = 1 * DOLLARS;
    pub const TermDuration: BlockNumber = 7 * DAYS;
    pub const DesiredMembers: u32 = 13;
    pub const DesiredRunnersUp: u32 = 7;
    pub const ElectionsPhragmenModuleId: LockIdentifier = *b"phrelect";
}

// Make sure that there are no more than `MAX_MEMBERS` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= pallet_collective::MAX_MEMBERS);

impl pallet_elections_phragmen::Trait for Runtime {
    type Event = Event;
    type ModuleId = ElectionsPhragmenModuleId;
    type Currency = Balances;
    type ChangeMembers = Council;
    // NOTE: this implies that council's genesis members cannot be set directly and must come from
    // this module.
    type InitializeMembers = Council;
    type CurrencyToVote = CurrencyToVoteHandler;
    type CandidacyBond = CandidacyBond;
    type VotingBond = VotingBond;
    type LoserCandidate = ();
    type BadReport = ();
    type KickedMember = ();
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
}

parameter_types! {
    pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
    pub const TechnicalMaxProposals: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Trait<TechnicalCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
}

type EnsureRootOrHalfCouncil = EnsureOneOf<
    AccountId,
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
>;
impl pallet_membership::Trait<pallet_membership::Instance1> for Runtime {
    type Event = Event;
    type AddOrigin = EnsureRootOrHalfCouncil;
    type RemoveOrigin = EnsureRootOrHalfCouncil;
    type SwapOrigin = EnsureRootOrHalfCouncil;
    type ResetOrigin = EnsureRootOrHalfCouncil;
    type PrimeOrigin = EnsureRootOrHalfCouncil;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 1 * DAYS;
    pub const Burn: Permill = Permill::from_percent(50);
    pub const TipCountdown: BlockNumber = 1 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = 1 * DOLLARS;
    pub const TipReportDepositPerByte: Balance = 1 * CENTS;
    pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");
}

impl pallet_treasury::Trait for Runtime {
    type ModuleId = TreasuryModuleId;
    type Currency = Balances;
    type ApproveOrigin = EnsureOneOf<
        AccountId,
        EnsureRoot<AccountId>,
        pallet_collective::EnsureMembers<_4, AccountId, CouncilCollective>,
    >;
    type RejectOrigin = EnsureOneOf<
        AccountId,
        EnsureRoot<AccountId>,
        pallet_collective::EnsureMembers<_2, AccountId, CouncilCollective>,
    >;
    type Tippers = Elections;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type TipReportDepositPerByte = TipReportDepositPerByte;
    type Event = Event;
    type ProposalRejection = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * MaximumBlockWeight::get();
}

impl pallet_scheduler::Trait for Runtime {
    type Event = Event;
    type Origin = Origin;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
}

///////////////////////////////////////////
// Chainx pallets
///////////////////////////////////////////

impl xpallet_system::Trait for Runtime {
    type Event = Event;
}

impl xpallet_assets::Trait for Runtime {
    type Currency = Balances;
    type Event = Event;
    type OnAssetChanged = XMiningAsset;
    type OnAssetRegisterOrRevoke = XMiningAsset;
}

impl xpallet_gateway_records::Trait for Runtime {
    type Event = Event;
}

impl xpallet_gateway_common::Trait for Runtime {
    type Event = Event;
    type Validator = XStaking;
    type Bitcoin = XGatewayBitcoin;
    type BitcoinTrustee = XGatewayBitcoin;
}

impl xpallet_gateway_bitcoin::Trait for Runtime {
    type Event = Event;
    type AccountExtractor = xpallet_gateway_common::extractor::Extractor;
    type TrusteeSessionProvider = trustees::bitcoin::BtcTrusteeSessionManager<Runtime>;
    type TrusteeMultiSigProvider = trustees::bitcoin::BtcTrusteeMultisig<Runtime>;
    type Channel = XGatewayCommon;
}

impl xpallet_dex_spot::Trait for Runtime {
    type Event = Event;
    type Price = Balance;
}

impl xpallet_contracts::Trait for Runtime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Call = Call;
    type Event = Event;
    type DetermineContractAddress = xpallet_contracts::SimpleAddressDeterminer<Runtime>;
    type TrieIdGenerator = xpallet_contracts::TrieIdFromParentCounter<Runtime>;
    type StorageSizeOffset = xpallet_contracts::DefaultStorageSizeOffset;
    type MaxDepth = xpallet_contracts::DefaultMaxDepth;
    type MaxValueSize = xpallet_contracts::DefaultMaxValueSize;
    type WeightPrice = pallet_transaction_payment::Module<Self>;
}

pub struct SimpleTreasuryAccount;
impl xp_mining_staking::TreasuryAccount<AccountId> for SimpleTreasuryAccount {
    fn treasury_account() -> AccountId {
        TreasuryModuleId::get().into_account()
    }
}

impl xpallet_mining_staking::Trait for Runtime {
    type Event = Event;
    type Currency = Balances;
    type SessionDuration = SessionDuration;
    type SessionInterface = Self;
    type TreasuryAccount = SimpleTreasuryAccount;
    type AssetMining = ();
    type DetermineRewardPotAccount =
        xpallet_mining_staking::SimpleValidatorRewardPotAccountDeterminer<Runtime>;
}

impl xpallet_mining_asset::Trait for Runtime {
    type Event = Event;
    type StakingInterface = Self;
    type TreasuryAccount = SimpleTreasuryAccount;
    type DetermineRewardPotAccount =
        xpallet_mining_asset::SimpleAssetRewardPotAccountDeterminer<Runtime>;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = chainx_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Authorship: pallet_authorship::{Module, Call, Storage, Inherent},
        Aura: pallet_aura::{Module, Config<T>, Inherent(Timestamp)},
        Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
        Democracy: pallet_democracy::{Module, Call, Storage, Config, Event<T>},
        Council: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
        TechnicalCommittee: pallet_collective::<Instance2>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
        Elections: pallet_elections_phragmen::{Module, Call, Storage, Event<T>, Config<T>},
        TechnicalMembership: pallet_membership::<Instance1>::{Module, Call, Storage, Event<T>, Config<T>},
        Treasury: pallet_treasury::{Module, Call, Storage, Config, Event<T>},
        Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>},
        Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        ImOnline: pallet_im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
        Offences: pallet_offences::{Module, Call, Storage, Event},
        Utility: pallet_utility::{Module, Call, Event},
        Multisig: pallet_multisig::{Module, Call, Storage, Event<T>},
        Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},

        XSystem: xpallet_system::{Module, Call, Storage, Event<T>, Config},
        XAssets: xpallet_assets::{Module, Call, Storage, Event<T>, Config<T>},
        XGatewayRecords: xpallet_gateway_records::{Module, Call, Storage, Event<T>},
        XGatewayCommon: xpallet_gateway_common::{Module, Call, Storage, Event<T>, Config<T>},
        XGatewayBitcoin: xpallet_gateway_bitcoin::{Module, Call, Storage, Event<T>, Config},
        XContracts: xpallet_contracts::{Module, Call, Config, Storage, Event<T>},
        XStaking: xpallet_mining_staking::{Module, Call, Storage, Event<T>, Config<T>},
        XMiningAsset: xpallet_mining_asset::{Module, Call, Storage, Event<T>, Config<T>},

        XSpot: xpallet_dex_spot::{Module, Call, Storage, Event<T>, Config<T>},
    }
);

/// The address format for describing accounts.
pub type Address = AccountId;
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
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    BaseFilter,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllModules,
>;

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
            Runtime::metadata().into()
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

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> u64 {
            Aura::slot_duration()
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
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

        fn submit_report_equivocation_extrinsic(
            _equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            _authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            // NOTE: this is the only implementation possible since we've
            // defined our key owner proof type as a bottom type (i.e. a type
            // with no values).
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
        UncheckedExtrinsic,
    > for Runtime {
        fn query_info(uxt: UncheckedExtrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
    }

    impl xpallet_assets_rpc_runtime_api::AssetsApi<Block, AccountId, Balance> for Runtime {
        fn assets_for_account(who: AccountId) -> BTreeMap<AssetId, BTreeMap<AssetType, Balance>> {
            XAssets::valid_assets_of(&who)
        }

        fn assets() -> BTreeMap<AssetId, TotalAssetInfo<Balance>> {
            XAssets::total_asset_infos()
        }
    }

    impl xpallet_mining_staking_rpc_runtime_api::XStakingApi<Block, AccountId, Balance, BlockNumber> for Runtime {
        fn validators() -> Vec<ValidatorInfo<AccountId, RpcBalance<Balance>, BlockNumber>> {
            XStaking::validators_info()
        }
        fn validator_info_of(who: AccountId) -> ValidatorInfo<AccountId, RpcBalance<Balance>, BlockNumber> {
            XStaking::validator_info_of(who)
        }
        fn staking_dividend_of(who: AccountId) -> BTreeMap<AccountId, RpcBalance<Balance>> {
            XStaking::staking_dividend_of(who)
        }
        fn nomination_details_of(who: AccountId) -> BTreeMap<AccountId, RpcNominatorLedger<RpcBalance<Balance>, BlockNumber>> {
            XStaking::nomination_details_of(who)
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

    impl xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<Block, AccountId, Balance> for Runtime {
        fn withdrawal_limit(asset_id: AssetId) -> Result<WithdrawalLimit<Balance>, DispatchError> {
            XGatewayCommon::withdrawal_limit(&asset_id)
        }

        fn verify_withdrawal(asset_id: AssetId, value: Balance, addr: AddrStr, memo: Memo) -> Result<(), DispatchError> {
            XGatewayCommon::verify_withdrawal(asset_id, value, &addr, &memo)
        }

        fn trustee_multisigs() -> BTreeMap<Chain, AccountId> {
            XGatewayCommon::trustee_multisigs()
        }

        fn trustee_properties(chain: Chain, who: AccountId) -> Option<GenericTrusteeIntentionProps> {
            XGatewayCommon::trustee_intention_props_of(who, chain)
        }

        fn trustee_session_info(chain: Chain) -> Option<GenericTrusteeSessionInfo<AccountId>> {
            let number = match XGatewayCommon::trustee_session_info_len(chain).checked_sub(1) {
                Some(r) => r,
                None => u32::max_value(),
            };
            XGatewayCommon::trustee_session_info_of(chain, number)
        }

        fn generate_trustee_session_info(chain: Chain, candidates: Vec<AccountId>) -> Result<GenericTrusteeSessionInfo<AccountId>, DispatchError> {
            XGatewayCommon::try_generate_session_info(chain, candidates)
        }
    }

    impl xpallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber>
        for Runtime
    {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: u64,
            input_data: Vec<u8>,
        ) -> ContractExecResult {
            let exec_result =
                XContracts::bare_call(origin, dest.into(), value, gas_limit, input_data);
            match exec_result {
                Ok(v) => ContractExecResult::Success {
                    status: v.status,
                    data: v.data,
                },
                Err(_) => ContractExecResult::Error,
            }
        }

        fn get_storage(
            address: AccountId,
            key: [u8; 32],
        ) -> xpallet_contracts_primitives::GetStorageResult {
            XContracts::get_storage(address, key)
        }

        fn xrc20_call(
            id: AssetId,
            selector: XRC20Selector,
            data: Vec<u8>,
        ) -> ContractExecResult {
            let exec_result = XContracts::call_xrc20(id, selector, data);
            match exec_result {
                Ok(v) => ContractExecResult::Success {
                    status: v.status,
                    data: v.data,
                },
                Err(_) => ContractExecResult::Error,
            }
        }
    }
}
