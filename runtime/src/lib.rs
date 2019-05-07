// Copyright 2018-2019 Chainpool.

//! The ChainX runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]
mod fee;
mod trustee;
mod xexecutive;

use parity_codec::Decode;
use rstd::collections::btree_map::BTreeMap;
use rstd::prelude::*;

// substrate
use client::{
    block_builder::api::{self as block_builder_api, CheckInherentsResult, InherentData},
    impl_runtime_apis, runtime_api as client_api,
};
use runtime_primitives::generic;
use runtime_primitives::traits::{
    AuthorityIdFor, BlakeTwo256, Block as BlockT, DigestFor, NumberFor, StaticLookup,
};
use runtime_primitives::transaction_validity::TransactionValidity;
use runtime_primitives::ApplyResult;
pub use runtime_primitives::{create_runtime_str, Perbill, Permill};
use substrate_primitives::OpaqueMetadata;
pub use support::{construct_runtime, StorageValue};

pub use timestamp::BlockPeriod;
pub use timestamp::Call as TimestampCall;

#[cfg(any(feature = "std", test))]
use version::NativeVersion;
use version::RuntimeVersion;

// chainx
use chainx_primitives;
use runtime_api;
use xgrandpa::fg_primitives::{self, ScheduledChange};
use xr_primitives;

// chainx
use chainx_primitives::{
    Acceleration, AccountId, AccountIndex, AuthorityId, AuthoritySignature, Balance, BlockNumber,
    Hash, Index, Signature, Timestamp as TimestampU64,
};

pub use xaccounts;
pub use xassets;
pub use xbitcoin;
pub use xbridge_common;
pub use xbridge_features;

use xbridge_common::types::{GenericAllSessionInfo, GenericTrusteeIntentionProps};

#[cfg(any(feature = "std", test))]
pub use runtime_primitives::BuildStorage;

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("chainx"),
    impl_name: create_runtime_str!("chainx-net"),
    authoring_version: 1,
    spec_version: 2,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

impl system::Trait for Runtime {
    type Origin = Origin;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type Digest = generic::Digest<Log>;
    type AccountId = AccountId;
    type Lookup = Indices;
    type Header = Header;
    type Event = Event;
    type Log = Log;
}

impl timestamp::Trait for Runtime {
    type Moment = TimestampU64;
    type OnTimestampSet = Aura;
}

impl consensus::Trait for Runtime {
    type Log = Log;
    type SessionKey = AuthorityId;
    type InherentOfflineReport = ();
}

impl indices::Trait for Runtime {
    type AccountIndex = AccountIndex;
    type IsDeadAccount = XAssets;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = Event;
}

impl xsession::Trait for Runtime {
    type ConvertAccountIdToSessionKey = ();
    type OnSessionChange = (XStaking, xgrandpa::SyncedAuthorities<Runtime>);
    type Event = Event;
}

impl xgrandpa::Trait for Runtime {
    type Log = Log;
    type Event = Event;
}

impl xaura::Trait for Runtime {
    type HandleReport = xaura::StakingSlasher<Runtime>;
}

impl xbootstrap::Trait for Runtime {}

// xrml trait
impl xsystem::Trait for Runtime {
    type ValidatorList = Session;
    type Validator = XAccounts;
}

impl xaccounts::Trait for Runtime {
    type DetermineIntentionJackpotAccountId = xaccounts::SimpleAccountIdDeterminator<Runtime>;
}
// fees
impl xfee_manager::Trait for Runtime {
    type Event = Event;
}
// assets
impl xassets::Trait for Runtime {
    type Balance = Balance;
    type OnNewAccount = Indices;
    type Event = Event;
    type OnAssetChanged = (XTokens);
    type OnAssetRegisterOrRevoke = (XTokens, XSpot);
}

impl xrecords::Trait for Runtime {
    type Event = Event;
}

impl xprocess::Trait for Runtime {}

impl xstaking::Trait for Runtime {
    type Event = Event;
    type OnRewardCalculation = XTokens;
    type OnReward = XTokens;
}

impl xtokens::Trait for Runtime {
    type Event = Event;
    type DetermineTokenJackpotAccountId = xtokens::SimpleAccountIdDeterminator<Runtime>;
}

impl xspot::Trait for Runtime {
    type Price = Balance;
    type Event = Event;
}

// bridge
impl xbitcoin::Trait for Runtime {
    type AccountExtractor = xbridge_common::extractor::Extractor<AccountId>;
    type TrusteeSessionProvider = XBridgeFeatures;
    type TrusteeMultiSigProvider = xbridge_features::trustees::BitcoinTrusteeMultiSig<Runtime>;
    type CrossChainProvider = XBridgeFeatures;
    type Event = Event;
}

impl xsdot::Trait for Runtime {
    type AccountExtractor = xbridge_common::extractor::Extractor<AccountId>;
    type CrossChainProvider = XBridgeFeatures;
    type Event = Event;
}

impl xbridge_features::Trait for Runtime {
    type TrusteeMultiSig = xbridge_features::SimpleTrusteeMultiSigIdFor<Runtime>;
    type Event = Event;
}

impl xmultisig::Trait for Runtime {
    type MultiSig = xmultisig::SimpleMultiSigIdFor<Runtime>;
    type GenesisMultiSig = xmultisig::ChainXGenesisMultisig<Runtime>;
    type Proposal = Call;
    type Event = Event;
}

impl finality_tracker::Trait for Runtime {
    type OnFinalizationStalled = xgrandpa::SyncedAuthorities<Runtime>;
}

construct_runtime!(
    pub enum Runtime with Log(InternalLog: DigestItem<Hash, AuthorityId, AuthoritySignature>) where
        Block = Block,
        NodeBlock = chainx_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: system::{default, Log(ChangesTrieRoot)},
        Indices: indices::{Module, Call, Storage, Event<T>},
        Timestamp: timestamp::{Module, Call, Storage, Config<T>, Inherent},
        Consensus: consensus::{Module, Call, Storage, Config<T>, Log(AuthoritiesChange), Inherent},
        Session: xsession,
        FinalityTracker: finality_tracker::{Module, Call, Inherent},
        Grandpa: xgrandpa::{Module, Call, Storage, Log(), Event<T>},
        Aura: xaura::{Module, Inherent(Timestamp)},

        // chainx runtime module
        XSystem: xsystem::{Module, Call, Storage, Inherent, Config<T>},
        XAccounts: xaccounts::{Module, Strorage},
        // fee
        XFeeManager: xfee_manager::{Module, Call, Storage, Config<T>, Event<T>},
        // assets
        XAssets: xassets,
        XAssetsRecords: xrecords::{Module, Call, Storage, Event<T>},
        XAssetsProcess: xprocess::{Module, Call, Storage, Config<T>},
        // mining
        XStaking: xstaking,
        XTokens: xtokens::{Module, Call, Storage, Event<T>, Config<T>},
        // dex
        XSpot: xspot,
        // bridge
        XBridgeOfBTC: xbitcoin::{Module, Call, Storage, Config<T>, Event<T>},
        XBridgeOfSDOT: xsdot::{Module, Call, Storage, Config<T>, Event<T>},
        XBridgeFeatures: xbridge_features,
        // multisig
        XMultiSig: xmultisig::{Module, Call, Storage, Event<T>},

        XBootstrap: xbootstrap::{Config<T>},
    }
);

/// The address format for describing accounts.
pub type Address = <Indices as StaticLookup>::Source;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// Custom Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = xr_primitives::generic::UncheckedMortalCompactExtrinsic<
    Address,
    Index,
    Call,
    Signature,
    Acceleration,
>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Index, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive =
    xexecutive::Executive<Runtime, Block, system::ChainContext<Runtime>, XFeeManager, AllModules>;

impl_runtime_apis! {
    impl client_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }

        fn authorities() -> Vec<AuthorityId> {
            panic!("Deprecated, please use `AuthoritiesApi`.")
        }
    }

    impl client_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
        }
    }

    impl block_builder_api::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
            data.check_extrinsics(&block)
        }

        fn random_seed() -> <Block as BlockT>::Hash {
            System::random_seed()
        }
    }

    impl client_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
            Executive::validate_transaction(tx)
        }
    }

    impl offchain_primitives::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(number: NumberFor<Block>) {
            Executive::offchain_worker(number)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_pending_change(digest: &DigestFor<Block>)
            -> Option<ScheduledChange<NumberFor<Block>>>
        {
            for log in digest.logs.iter().filter_map(|l| match l {
                Log(InternalLog::xgrandpa(grandpa_signal)) => Some(grandpa_signal),
                _=> None
            }) {
                if let Some(change) = Grandpa::scrape_digest_change(log) {
                    return Some(change);
                }
            }
            None
        }

        fn grandpa_forced_change(digest: &DigestFor<Block>)
            -> Option<(NumberFor<Block>, ScheduledChange<NumberFor<Block>>)>
        {
            for log in digest.logs.iter().filter_map(|l| match l {
                Log(InternalLog::xgrandpa(grandpa_signal)) => Some(grandpa_signal),
                _ => None
            }) {
                if let Some(change) = Grandpa::scrape_digest_forced_change(log) {
                    return Some(change);
                }
            }
            None
        }

        fn grandpa_authorities() -> Vec<(AuthorityId, u64)> {
            Grandpa::grandpa_authorities()
        }
    }

    impl consensus_aura::AuraApi<Block> for Runtime {
        fn slot_duration() -> u64 {
            Aura::slot_duration()
        }
    }

    impl consensus_authorities::AuthoritiesApi<Block> for Runtime {
        fn authorities() -> Vec<AuthorityIdFor<Block>> {
            Consensus::authorities()
        }
    }

    impl runtime_api::xassets_api::XAssetsApi<Block> for Runtime {
        fn valid_assets() -> Vec<xassets::Token> {
            XAssets::valid_assets()
        }

        fn all_assets() -> Vec<(xassets::Asset, bool)> {
            XAssets::all_assets()
        }

        fn valid_assets_of(who: AccountId) -> Vec<(xassets::Token, BTreeMap<xassets::AssetType, Balance>)> {
            XAssets::valid_assets_of(&who)
        }

        fn withdrawal_list_of(chain: xassets::Chain) -> Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, TimestampU64>> {
            match chain {
                xassets::Chain::Bitcoin => XBridgeOfBTC::withdrawal_list(),
                xassets::Chain::Ethereum => Vec::new(),
                _ => Vec::new(),
            }
        }

        fn deposit_list_of(chain: xassets::Chain) -> Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, TimestampU64>> {
            match chain {
                xassets::Chain::Bitcoin => XBridgeOfBTC::deposit_list(),
                xassets::Chain::Ethereum => Vec::new(),
                _ => Vec::new(),
            }
        }

        fn verify_address(token: xassets::Token, addr: xrecords::AddrStr, ext: xassets::Memo) -> Result<(), Vec<u8>> {
            XAssetsProcess::verify_address(token, addr, ext).map_err(|e| e.as_bytes().to_vec())
        }

        fn minimal_withdrawal_value(token: xassets::Token) -> Option<Balance> {
            XAssetsProcess::minimal_withdrawal_value(&token)
        }
    }

    impl runtime_api::xmining_api::XMiningApi<Block> for Runtime {
        fn jackpot_accountid_for(who: AccountId) -> AccountId {
            XStaking::jackpot_accountid_for(&who)
        }
        fn multi_jackpot_accountid_for(whos: Vec<AccountId>) -> Vec<AccountId> {
            XStaking::multi_jackpot_accountid_for(&whos)
        }
        fn token_jackpot_accountid_for(token: xassets::Token) -> AccountId {
            XTokens::token_jackpot_accountid_for(&token)
        }
        fn multi_token_jackpot_accountid_for(tokens: Vec<xassets::Token>) -> Vec<AccountId> {
            XTokens::multi_token_jackpot_accountid_for(&tokens)
        }
        fn asset_power(token: xassets::Token) -> Option<Balance> {
            XTokens::asset_power(&token)
        }
    }

    impl runtime_api::xspot_api::XSpotApi<Block> for Runtime {
        fn aver_asset_price(token: xassets::Token) -> Option<Balance> {
            XSpot::aver_asset_price(&token)
        }
    }

    impl runtime_api::xfee_api::XFeeApi<Block> for Runtime {
        fn transaction_fee(call_params: Vec<u8>, encoded_len: u64) -> Option<u64> {
            use fee::CheckFee;

            let call: Call = if let Some(call) = Decode::decode(&mut call_params.as_slice()) {
                call
            } else {
                return None;
            };

            let switch = xfee_manager::SwitchStore::default();
            call.check_fee(switch).map(|power|
                XFeeManager::transaction_fee(power, encoded_len)
            )
        }
    }

    impl runtime_api::xsession_api::XSessionApi<Block> for Runtime {
        fn pubkeys_for_validator_name(name: Vec<u8>) -> Option<(AccountId, Option<AuthorityId>)> {
            Session::pubkeys_for_validator_name(name)
        }
    }

    impl runtime_api::xbridge_api::XBridgeApi<Block> for Runtime {
        fn mock_new_trustees(chain: xassets::Chain, candidates: Vec<AccountId>) -> Result<GenericAllSessionInfo<AccountId>, Vec<u8>> {
            XBridgeFeatures::mock_trustee_session_impl(chain, candidates).map_err(|e| e.as_bytes().to_vec())
        }
        fn trustee_props_for(who: AccountId) ->  BTreeMap<xassets::Chain, GenericTrusteeIntentionProps> {
            XBridgeFeatures::trustee_props_for(&who)
        }
        fn trustee_session_info() -> BTreeMap<xassets::Chain, GenericAllSessionInfo<AccountId>> {
            let mut map = BTreeMap::new();
            for chain in xassets::Chain::iterator() {
                if let Some((_, info)) = Self::trustee_session_info_for(*chain) {
                    map.insert(*chain, info);
                }
            }
            map
        }
        fn trustee_session_info_for(chain: xassets::Chain) -> Option<(u32, GenericAllSessionInfo<AccountId>)> {
            XBridgeFeatures::current_trustee_session_info_for(chain).map(|info| (
                (XBridgeFeatures::current_session_number(chain), info)
            ))
        }
    }
}
