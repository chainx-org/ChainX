// Copyright 2018 Chainpool.

//! The ChainX runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]

#[cfg(test)]
#[macro_use]
extern crate hex_literal;

#[cfg(test)]
extern crate serde;

extern crate parity_codec as codec;

#[macro_use]
extern crate substrate_client as client;
extern crate substrate_consensus_aura_primitives as consensus_aura;
extern crate substrate_primitives as primitives;

#[macro_use]
extern crate sr_primitives as runtime_primitives;
#[macro_use]
extern crate sr_version as version;
extern crate sr_io as runtime_io;
extern crate sr_std as rstd;

// substrate runtime module
#[macro_use]
extern crate srml_support;
extern crate srml_balances as balances;
extern crate srml_consensus as consensus;
extern crate srml_indices as indices;
extern crate srml_sudo as sudo;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;
extern crate xrml_aura as aura;
extern crate xrml_grandpa as grandpa;
extern crate xrml_session as session;
// unused
//extern crate srml_contract as contract;
//extern crate srml_council as council;
//extern crate srml_democracy as democracy;
//extern crate srml_treasury as treasury;

// chainx
extern crate chainx_primitives;
extern crate xr_primitives;

extern crate runtime_api;

// chainx runtime module
extern crate xrml_xsupport as xsupport;

pub extern crate xrml_xaccounts as xaccounts;
pub extern crate xrml_xbootstrap as xbootstrap;
pub extern crate xrml_xsystem as xsystem;
// fee;
pub extern crate xrml_fee_manager as fee_manager;
// assets;
pub extern crate xrml_xassets_assets as xassets;
pub extern crate xrml_xassets_process as xprocess;
pub extern crate xrml_xassets_records as xrecords;
// bridge
pub extern crate xrml_bridge_bitcoin as bitcoin;
pub extern crate xrml_bridge_sdot as sdot;
// staking
pub extern crate xrml_mining_staking as xstaking;
pub extern crate xrml_mining_tokens as xtokens;

// dex
pub extern crate xrml_xdex_spot as xspot;
extern crate xrml_xmultisig as xmultisig;

mod fee;
mod xexecutive;

use rstd::prelude::*;
// substrate
use primitives::OpaqueMetadata;
use runtime_primitives::generic;
use runtime_primitives::traits::{
    BlakeTwo256, Block as BlockT, Convert, DigestFor, NumberFor, StaticLookup,
};
//#[cfg(feature = "std")]
//use council::{motions as council_motions, voting as council_voting};
use client::{
    block_builder::api::{self as block_builder_api, CheckInherentsResult, InherentData},
    runtime_api as client_api,
};
use runtime_primitives::transaction_validity::TransactionValidity;
use runtime_primitives::ApplyResult;
#[cfg(any(feature = "std", test))]
use version::NativeVersion;
use version::RuntimeVersion;

use grandpa::fg_primitives::{self, ScheduledChange};
pub use runtime_primitives::{Perbill, Permill};

// for set consensus period
pub use srml_support::StorageValue;
pub use timestamp::BlockPeriod;
pub use timestamp::Call as TimestampCall;

// chainx
use chainx_primitives::{
    Acceleration, AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, SessionKey,
    Signature, Timestamp as TimestampU64,
};
// chainx runtime
// xassets
//pub use xassets;
// xbitcoin
//pub use xbitcoin;
#[cfg(feature = "std")]
pub use bitcoin::Params;

#[cfg(any(feature = "std", test))]
pub use runtime_primitives::BuildStorage;

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("chainx"),
    impl_name: create_runtime_str!("chainx-net"),
    authoring_version: 1,
    spec_version: 1,
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

impl balances::Trait for Runtime {
    type Balance = Balance;
    type OnNewAccount = Indices;
    type OnFreeBalanceZero = ();
    type EnsureAccountLiquid = ();
    type Event = Event;
}

impl timestamp::Trait for Runtime {
    type Moment = TimestampU64;
    type OnTimestampSet = Aura;
}

impl consensus::Trait for Runtime {
    type Log = Log;
    type SessionKey = SessionKey;
    type InherentOfflineReport = ();
}

/// Session key conversion.
pub struct SessionKeyConversion;

impl Convert<AccountId, SessionKey> for SessionKeyConversion {
    fn convert(a: AccountId) -> SessionKey {
        a.to_fixed_bytes().into()
    }
}

impl session::Trait for Runtime {
    type ConvertAccountIdToSessionKey = SessionKeyConversion;
    type OnSessionChange = (XStaking, grandpa::SyncedAuthorities<Runtime>);
    type Event = Event;
}

impl grandpa::Trait for Runtime {
    type Log = Log;
    type Event = Event;
}

impl aura::Trait for Runtime {
    type HandleReport = aura::StakingSlasher<Runtime>;
}

// bridge
impl bitcoin::Trait for Runtime {
    type Event = Event;
}

impl sdot::Trait for Runtime {
    type Event = Event;
}

//impl treasury::Trait for Runtime {
//    type ApproveOrigin = council_motions::EnsureMembers<_4>;
//    type RejectOrigin = council_motions::EnsureMembers<_2>;
//    type Event = Event;
//}
//
//impl democracy::Trait for Runtime {
//    type Proposal = Call;
//    type Event = Event;
//}
//
//impl council::Trait for Runtime {
//    type Event = Event;
//}
//
//impl contract::Trait for Runtime {
//    type DetermineContractAddress = contract::SimpleAddressDeterminator<Runtime>;
//    type Gas = u64;
//    type Event = Event;
//}
//
//// TODO add voting and motions at here
//impl council::voting::Trait for Runtime {
//    type Event = Event;
//}
//
//impl council::motions::Trait for Runtime {
//    type Origin = Origin;
//    type Proposal = Call;
//    type Event = Event;
//}

impl xbootstrap::Trait for Runtime {}

// cxrml trait
impl xsystem::Trait for Runtime {
    type ValidatorList = Session;
    type Validator = XAccounts;
}

impl xaccounts::Trait for Runtime {
    type Event = Event;
    type DetermineIntentionJackpotAccountId = xaccounts::SimpleAccountIdDeterminator<Runtime>;
}
// fees
impl fee_manager::Trait for Runtime {
    //    type Event = Event;
}
// assets
impl xassets::Trait for Runtime {
    type Event = Event;
    type OnAssetChanged = (XTokens);
    type OnAssetRegisterOrRevoke = (XTokens, XSpot);
}

impl xrecords::Trait for Runtime {
    type Event = Event;
}

impl xprocess::Trait for Runtime {}

impl xstaking::Trait for Runtime {
    type OnRewardCalculation = XTokens;
    type OnReward = XTokens;
    type Event = Event;
}

impl xtokens::Trait for Runtime {
    type Event = Event;
    type DetermineTokenJackpotAccountId = xtokens::SimpleAccountIdDeterminator<Runtime>;
}

impl xspot::Trait for Runtime {
    type Event = Event;
    type Price = Balance;
}

impl indices::Trait for Runtime {
    type AccountIndex = AccountIndex;
    type IsDeadAccount = Balances;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = Event;
}

impl sudo::Trait for Runtime {
    type Event = Event;
    type Proposal = Call;
}

impl xmultisig::Trait for Runtime {
    type MultiSig = xmultisig::SimpleMultiSigIdFor<Runtime>;
    type GenesisMultiSig = xmultisig::ChainXGenesisMultisig<Runtime>;
    type Proposal = Call;
    type Event = Event;
}

construct_runtime!(
    pub enum Runtime with Log(InternalLog: DigestItem<Hash, SessionKey>) where
        Block = Block,
        NodeBlock = chainx_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: system::{default, Log(ChangesTrieRoot)},
        Indices: indices,
        Balances: balances::{Module, Storage, Config<T>, Event<T>},
        Timestamp: timestamp::{Module, Call, Storage, Config<T>, Inherent},
        Consensus: consensus::{Module, Call, Storage, Config<T>, Log(AuthoritiesChange), Inherent},
        Session: session,
        Grandpa: grandpa::{Module, Call, Storage, Log(), Event<T>},
        Aura: aura::{Module, Inherent(Timestamp)},
        Sudo: sudo,

        // chainx runtime module
        XSystem: xsystem::{Module, Call, Storage, Config<T>, Inherent}, //, Inherent},
        XAccounts: xaccounts::{Module, Storage, Event<T>}, //, Inherent},
        // fee
        XFeeManager: fee_manager::{Module, Call, Storage, Config<T>},
        // assets
        XAssets: xassets,
        XAssetsRecords: xrecords::{Module, Storage, Event<T>},
        XAssetsProcess: xprocess::{Module, Call, Storage, Config<T>},
        // mining
        XStaking: xstaking,
        XTokens: xtokens::{Module, Call, Storage, Event<T>, Config<T>},
        // dex
        XSpot: xspot,
        // bridge
        XBridgeOfBTC: bitcoin::{Module, Call, Storage, Config<T>, Event<T>},
        XBridgeOfSDOT: sdot::{Module, Call, Storage, Config<T>, Event<T>},
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
/// Executive: handles dispatch to the various modules.
pub type Executive =
    xexecutive::Executive<Runtime, Block, system::ChainContext<Runtime>, XFeeManager, AllModules>;

// define tokenbalances module type
//pub type TokenBalance = u128;

impl_runtime_apis! {
    impl client_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn authorities() -> Vec<SessionKey> {
            Consensus::authorities()
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialise_block(header: &<Block as BlockT>::Header) {
            Executive::initialise_block(header)
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

        fn finalise_block() -> <Block as BlockT>::Header {
            Executive::finalise_block()
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

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_pending_change(digest: &DigestFor<Block>)
            -> Option<ScheduledChange<NumberFor<Block>>>
        {
            for log in digest.logs.iter().filter_map(|l| match l {
                Log(InternalLog::grandpa(grandpa_signal)) => Some(grandpa_signal),
                _=> None
            }) {
                if let Some(change) = Grandpa::scrape_digest_change(log) {
                    return Some(change);
                }
            }
            None
        }

        fn grandpa_authorities() -> Vec<(SessionKey, u64)> {
            Grandpa::grandpa_authorities()
        }
    }

    impl consensus_aura::AuraApi<Block> for Runtime {
        fn slot_duration() -> u64 {
            Aura::slot_duration()
        }
    }

    impl runtime_api::xassets_api::XAssetsApi<Block> for Runtime {
        fn valid_assets() -> Vec<xassets::Token> {
            XAssets::valid_assets()
        }

        fn all_assets() -> Vec<(xassets::Asset, bool)> {
            XAssets::all_assets()
        }

        fn valid_assets_of(who: AccountId) -> Vec<(xassets::Token, xsupport::storage::btree_map::CodecBTreeMap<xassets::AssetType, Balance>)> {
            XAssets::valid_assets_of(&who)
        }

        fn withdrawal_list_of(chain: xassets::Chain) -> Vec<xrecords::Application<AccountId, Balance, TimestampU64>> {
            XAssetsRecords::withdrawal_applications(chain)
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
            use codec::Decode;

            let call: Call = if let Some(call) = Decode::decode(&mut call_params.as_slice()) {
                call
            } else {
                return None;
            };

            let switch = fee_manager::SwitchStore::default();
            call.check_fee(switch).map(|power|
                XFeeManager::transaction_fee(power, encoded_len)
            )
        }
    }

    impl runtime_api::xsession_api::XSessionApi<Block> for Runtime {
        fn pubkeys_for_validator_name(name: Vec<u8>) -> Option<(AccountId, Option<SessionKey>)> {
            Session::pubkeys_for_validator_name(name)
        }
    }
}
