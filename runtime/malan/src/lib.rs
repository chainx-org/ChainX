// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

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
use sp_core::{
    crypto::KeyTypeId,
    u32_trait::{_1, _2, _3, _4, _5},
    OpaqueMetadata,
};
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

use frame_support::PalletId;
use frame_system::{EnsureOneOf, EnsureRoot, EnsureSignedBy};
use pallet_grandpa::fg_primitives;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots;

use chainx_runtime_common::{BlockLength, BlockWeights};
use xpallet_dex_spot::{Depth, FullPairInfo, RpcOrder, TradingPairId};
use xpallet_mining_asset::{MinerLedger, MiningAssetInfo, MiningDividendInfo};
use xpallet_mining_staking::{NominatorInfo, NominatorLedger, ValidatorInfo};
use xpallet_support::traits::MultisigAddressFor;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, debug, parameter_types,
    traits::{
        Contains, Currency, Get, Imbalance, InstanceFilter, KeyOwnerProofSystem, LockIdentifier,
        OnUnbalanced, Randomness,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        IdentityFee, Weight,
    },
    StorageValue,
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
    hash_rev, BtcHeader, BtcNetwork, BtcParams, BtcTxVerifier, Compact as BtcCompact,
    H256 as BtcHash,
};
pub use xpallet_gateway_common::{
    trustees,
    types::{GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, TrusteeInfoConfig},
};
pub use xpallet_gateway_records::Withdrawal;
pub use xpallet_mining_asset::MiningWeight;
pub use xpallet_mining_staking::VoteWeight;

/// Constant values used within the runtime.
pub mod constants;
/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
mod migrations;

use self::constants::{currency::*, time::*};
use self::impls::{ChargeExtraFee, DealWithFees, SlowAdjustingFeeUpdate};
use self::migrations::*;

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("chainx"),
    impl_name: create_runtime_str!("chainx-malan"),
    authoring_version: 1,
    spec_version: 12,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 3,
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

#[derive(Debug, Clone, Eq, PartialEq, codec::Encode, codec::Decode, TypeInfo)]
pub struct BaseFilter;
impl Contains<Call> for BaseFilter {
    fn contains(call: &Call) -> bool {
        use frame_support::dispatch::GetCallMetadata;

        if let Call::Currencies(_) = call {
            return false;
        }

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

    type KeyOwnerProofSystem = Historical;

    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::IdentificationTuple;

    type HandleEquivocation =
        pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;

    type WeightInfo = ();
    type DisabledValidators = Session;
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
    type HandleEquivocation = ();

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
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
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
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type ReserveIdentifier = [u8; 8];
    type MaxReserves = MaxReserves;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MILLICENTS; // 100 => 0.000001 pcx
    pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

impl xpallet_transaction_fee::Config for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_BLOCKS;
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
    /// We prioritize im-online heartbeats over election solution submission.
    pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
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
        XStaking::active_validator_set().collect()
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
    pub const CooloffPeriod: BlockNumber = 7 * DAYS;
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
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
        EnsureRoot<AccountId>,
    >;
    type BlacklistOrigin = EnsureRoot<AccountId>;
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
    type VoteLockingPeriod = EnactmentPeriod;
}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
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
    pub const DesiredMembers: u32 = 11;
    pub const DesiredRunnersUp: u32 = 7;
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
    pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
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
    AccountId,
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
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
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type ApproveOrigin = EnsureOneOf<
        AccountId,
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
    >;
    type RejectOrigin = EnsureOneOf<
        AccountId,
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>,
    >;
    type Event = Event;
    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type SpendPeriod = SpendPeriod;
    type Burn = NoBurn;
    type BurnDestination = ();
    type SpendFunds = Bounties;
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type MaxApprovals = MaxApprovals;
}

impl pallet_bounties::Config for Runtime {
    type Event = Event;
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type BountyCuratorDeposit = BountyCuratorDeposit;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
}

impl pallet_tips::Config for Runtime {
    type Event = Event;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type Tippers = Elections;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * MaximumBlockWeight::get();
    pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
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
// orml
///////////////////////////////////////////
use orml_currencies::BasicCurrencyAdapter;

impl orml_currencies::Config for Runtime {
    type Event = Event;
    type MultiCurrency = XAssets;
    type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
    type GetNativeCurrencyId = ChainXAssetId;
    type WeightInfo = ();
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
    type Amount = Amount;
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
    type Bitcoin = XGatewayBitcoin;
    type BitcoinTrustee = XGatewayBitcoin;
    type WeightInfo = xpallet_gateway_common::weights::SubstrateWeight<Runtime>;
}

impl xpallet_gateway_bitcoin::Config for Runtime {
    type Event = Event;
    type UnixTime = Timestamp;
    type AccountExtractor = xp_gateway_bitcoin::OpReturnExtractor;
    type TrusteeSessionProvider = trustees::bitcoin::BtcTrusteeSessionManager<Runtime>;
    type TrusteeOrigin = EnsureSignedBy<trustees::bitcoin::BtcTrusteeMultisig<Runtime>, AccountId>;
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
    fn treasury_account() -> AccountId {
        TreasuryPalletId::get().into_account()
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
        Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event} = 12,
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

        // orml
        // we retain Currencies Call for this call may be used in future, but we do not need this now,
        // so that we filter it in BaseFilter.
        Currencies: orml_currencies::{Pallet, Call, Event<T>} = 34,

        // It might be possible to merge this module into pallet_transaction_payment in future, thus
        // we put it at the end for keeping the extrinsic ordering.
        XTransactionFee: xpallet_transaction_fee::{Pallet, Event<T>} = 35,

        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 36,

        Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 37,
        Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 38,

        // Put Sudo last so that the extrinsic ordering stays the same once it's removed.
        Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 39,
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
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPallets,
    CustomOnRuntimeUpgrades,
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
            if let Some(extra_fee) = ChargeExtraFee::has_extra_fee(&uxt.function) {
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
            let maybe_extra = ChargeExtraFee::has_extra_fee(&uxt.function);
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

    impl xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<Block, AccountId, Balance> for Runtime {
        fn bound_addrs(who: AccountId) -> BTreeMap<Chain, Vec<ChainAddress>> {
            XGatewayCommon::bound_addrs(&who)
        }

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
            let number = XGatewayCommon::trustee_session_info_len(chain)
                .checked_sub(1)
                .unwrap_or_else(u32::max_value);
            XGatewayCommon::trustee_session_info_of(chain, number)
        }

        fn generate_trustee_session_info(chain: Chain, candidates: Vec<AccountId>) -> Result<GenericTrusteeSessionInfo<AccountId>, DispatchError> {
            let info = XGatewayCommon::try_generate_session_info(chain, candidates)?;
            // check multisig address
            let _ = XGatewayCommon::generate_multisig_addr(chain, &info)?;
            Ok(info)
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
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            impl frame_system_benchmarking::Config for Runtime {}

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

            add_benchmark!(params, batches, xpallet_assets, XAssets);
            add_benchmark!(params, batches, xpallet_assets_registrar, XAssetsRegistrar);
            add_benchmark!(params, batches, xpallet_mining_asset, XMiningAsset);
            add_benchmark!(params, batches, xpallet_mining_staking, XStaking);
            add_benchmark!(params, batches, xpallet_gateway_records, XGatewayRecords);
            add_benchmark!(params, batches, xpallet_gateway_common, XGatewayCommon);
            add_benchmark!(params, batches, xpallet_gateway_bitcoin, XGatewayBitcoin);
            add_benchmark!(params, batches, xpallet_dex_spot, XSpot);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

}
