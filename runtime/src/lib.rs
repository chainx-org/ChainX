// Copyright 2018-2019 Chainpool.

//! The ChainX runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]

#[macro_use]
mod fee;
mod tests;
mod trustee;
mod xcontracts_fee;
mod xexecutive;

use parity_codec::{Decode, Encode};
use rstd::collections::btree_map::BTreeMap;
use rstd::prelude::*;
use rstd::result;

// substrate
use client::{
    block_builder::api::{self as block_builder_api, CheckInherentsResult, InherentData},
    impl_runtime_apis, runtime_api as client_api,
};
use runtime_primitives::traits::{
    AuthorityIdFor, BlakeTwo256, Block as BlockT, DigestFor, NumberFor, StaticLookup, Zero,
};
use runtime_primitives::transaction_validity::TransactionValidity;
pub use runtime_primitives::{create_runtime_str, Perbill, Permill};
use runtime_primitives::{generic, ApplyResult};
use substrate_primitives::OpaqueMetadata;
use substrate_primitives::H512;
pub use support::{construct_runtime, parameter_types, StorageValue};

pub use timestamp::BlockPeriod;
pub use timestamp::Call as TimestampCall;

#[cfg(any(feature = "std", test))]
use version::NativeVersion;
use version::RuntimeVersion;

// chainx
use chainx_primitives;
use runtime_api;
use xgrandpa::fg_primitives::{self, ScheduledChange};
pub use xr_primitives::{AddrStr, ContractExecResult, GetStorageError, GetStorageResult};

// chainx
use chainx_primitives::{
    Acceleration, AccountId, AccountIndex, AuthorityId, AuthoritySignature, Balance, BlockNumber,
    Hash, Index, Signature, Timestamp as TimestampU64,
};

use fee::CheckFee;
use xcontracts_fee::XContractsCheckFee;

pub use xaccounts;
pub use xassets;
pub use xbitcoin;
pub use xbitcoin::lockup as xbitcoin_lockup;
pub use xbridge_common;
pub use xbridge_features;
pub use xcontracts::{self, XRC20Selector}; // re-export
pub use xprocess;

#[cfg(feature = "std")]
pub use xbootstrap::{self, ChainSpec};

use xsupport::ensure_with_errorlog;
#[cfg(feature = "std")]
use xsupport::u8array_to_hex;

use xbridge_common::types::{GenericAllSessionInfo, GenericTrusteeIntentionProps};

#[cfg(any(feature = "std", test))]
pub use runtime_primitives::BuildStorage;

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("chainx"),
    impl_name: create_runtime_str!("chainx-net"),
    authoring_version: 1,
    spec_version: 7,
    impl_version: 7,
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
    type DetermineTokenJackpotAccountId = xassets::SimpleAccountIdDeterminator<Runtime>;
}

impl xrecords::Trait for Runtime {
    type Event = Event;
}

impl xfisher::Trait for Runtime {
    type Event = Event;
    type CheckHeader = HeaderChecker;
}

impl xprocess::Trait for Runtime {}

impl xstaking::Trait for Runtime {
    type Event = Event;
    type OnRewardCalculation = XTokens;
    type OnReward = XTokens;
}

impl xtokens::Trait for Runtime {
    type Event = Event;
}

impl xspot::Trait for Runtime {
    type Price = Balance;
    type Event = Event;
}

// bridge
impl xbridge_common::Trait for Runtime {
    type Event = Event;
}

impl xbitcoin::Trait for Runtime {
    type XBitcoinLockup = Self;
    type AccountExtractor = xbridge_common::extractor::Extractor<AccountId>;
    type TrusteeSessionProvider = XBridgeFeatures;
    type TrusteeMultiSigProvider = xbridge_features::trustees::BitcoinTrusteeMultiSig<Runtime>;
    type CrossChainProvider = XBridgeFeatures;
    type Event = Event;
}

impl xbitcoin_lockup::Trait for Runtime {
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

// TODO
parameter_types! {
    pub const TombstoneDeposit: Balance = 1 * 10000000;
    pub const RentByteFee: Balance = 1 * 10000000;
    pub const RentDepositOffset: Balance = 1000 * 10000000;
    pub const SurchargeReward: Balance = 150 * 10000000;
}

pub struct DispatchFeeComputor;
impl
    xcontracts::ComputeDispatchFee<
        <Runtime as xcontracts::Trait>::Call,
        <Runtime as xassets::Trait>::Balance,
    > for DispatchFeeComputor
{
    fn compute_dispatch_fee(
        call: &<Runtime as xcontracts::Trait>::Call,
    ) -> Option<<Runtime as xassets::Trait>::Balance> {
        let switch = xfee_manager::Module::<Runtime>::switcher();
        let method_call_weight = XFeeManager::method_call_weight();
        let encoded_len = call.using_encoded(|encoded| encoded.len() as u64);
        (*call)
            .check_xcontracts_fee(switch, method_call_weight)
            .map(|weight| XFeeManager::transaction_fee(weight, encoded_len))
    }
}

impl xcontracts::Trait for Runtime {
    type Call = Call;
    type Event = Event;
    type DetermineContractAddress = xcontracts::SimpleAddressDeterminator<Runtime>;
    type ComputeDispatchFee = DispatchFeeComputor; //<Runtime>;
    type TrieIdGenerator = xcontracts::TrieIdFromParentCounter<Runtime>;
    type SignedClaimHandicap = xcontracts::DefaultSignedClaimHandicap;
    type TombstoneDeposit = TombstoneDeposit;
    type StorageSizeOffset = xcontracts::DefaultStorageSizeOffset;
    type RentByteFee = RentByteFee;
    type RentDepositOffset = RentDepositOffset;
    type MaxDepth = xcontracts::DefaultMaxDepth;
    type MaxValueSize = xcontracts::DefaultMaxValueSize;
    type BlockGasLimit = xcontracts::DefaultBlockGasLimit;
}

pub struct HeaderChecker;
impl xfisher::CheckHeader<AuthorityId, BlockNumber> for HeaderChecker {
    fn check_header(
        signer: &AuthorityId,
        first: &(xfisher::RawHeader, u64, H512),
        second: &(xfisher::RawHeader, u64, H512),
    ) -> result::Result<(BlockNumber, BlockNumber), &'static str> {
        if (*first).1 != (*second).1 {
            return Err("slot number not same");
        }

        let fst_header = verify_header(first, signer)?;
        let snd_header = verify_header(second, signer)?;
        if fst_header.hash() == snd_header.hash() {
            return Err("same header, do nothing for this");
        }
        Ok((fst_header.number, snd_header.number))
    }
}
fn verify_header(
    header: &(xfisher::RawHeader, u64, H512),
    expected_author: &AuthorityId,
) -> result::Result<Header, &'static str> {
    // hard code, digest with other type can't be decode in runtime, thus just can decode pre header(header without digest)
    // 3 * hash + vec<u8> + CompactNumber
    ensure_with_errorlog!(
        header.0.as_slice().len() <= 3 * 32 + 1 + 16,
        "should use pre header",
        "should use pre header|current len:{:?}",
        header.0.as_slice().len()
    );

    let pre_header: Header = Decode::decode(&mut header.0.as_slice()).ok_or("decode header err")?;

    // verify sign
    let to_sign = ((*header).1, pre_header.hash()).encode();

    ensure_with_errorlog!(
        runtime_io::ed25519_verify(&(header.2).0, &to_sign[..], expected_author.clone()),
        "check signature failed",
        "check signature failed|slot:{:}|pre_hash:{:?}|to_sign:{:}|sig:{:?}|author:{:?}",
        (*header).1,
        pre_header.hash(),
        u8array_to_hex(&to_sign[..]),
        header.2,
        expected_author
    );

    Ok(pre_header)
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
        XAssetsProcess: xprocess::{Module, Call, Storage},
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

        // fisher
        XFisher: xfisher::{Module, Call, Storage, Event<T>},

        XBridgeCommon: xbridge_common::{Module, Storage, Event<T>},
        XBridgeOfBTCLockup: xbitcoin_lockup::{Module, Call, Storage, Event<T>},

        XContracts: xcontracts,
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

//        fn authorities() -> Vec<AuthorityId> {
//            panic!("Deprecated, please use `AuthoritiesApi`.")
//        }
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

        fn verify_address(token: xassets::Token, addr: AddrStr, ext: xassets::Memo) -> Result<(), Vec<u8>> {
            XAssetsProcess::verify_address(token, addr, ext).map_err(|e| e.as_bytes().to_vec())
        }

        fn withdrawal_limit(token: xassets::Token) -> Option<xprocess::WithdrawalLimit<Balance>> {
            XAssetsProcess::withdrawal_limit(&token)
        }
    }

    impl runtime_api::xmining_api::XMiningApi<Block> for Runtime {
        fn jackpot_accountid_for_unsafe(who: AccountId) -> AccountId {
            XStaking::jackpot_accountid_for_unsafe(&who)
        }
        fn multi_jackpot_accountid_for_unsafe(whos: Vec<AccountId>) -> Vec<AccountId> {
            XStaking::multi_jackpot_accountid_for_unsafe(&whos)
        }
        fn token_jackpot_accountid_for_unsafe(token: xassets::Token) -> AccountId {
            XTokens::token_jackpot_accountid_for_unsafe(&token)
        }
        fn multi_token_jackpot_accountid_for_unsafe(tokens: Vec<xassets::Token>) -> Vec<AccountId> {
            XTokens::multi_token_jackpot_accountid_for_unsafe(&tokens)
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
            let call: Call = if let Some(call) = Decode::decode(&mut call_params.as_slice()) {
                call
            } else {
                return None;
            };

            let switch = xfee_manager::Module::<Runtime>::switcher();
            let method_call_weight = XFeeManager::method_call_weight();
            call.check_fee(switch, method_call_weight).map(|weight|
                XFeeManager::transaction_fee(weight, encoded_len)
            )
        }

        fn fee_weight_map() -> BTreeMap<Vec<u8>, u64> {
            let method_call_weight = XFeeManager::method_call_weight();
            fee::call_weight_map(&method_call_weight)
        }
    }

    impl runtime_api::xsession_api::XSessionApi<Block> for Runtime {
        fn pubkeys_for_validator_name(name: Vec<u8>) -> Option<(AccountId, Option<AuthorityId>)> {
            Session::pubkeys_for_validator_name(name)
        }
    }

    impl runtime_api::xstaking_api::XStakingApi<Block> for Runtime {
        fn intention_set() -> Vec<AccountId> {
            XStaking::intention_set()
        }
        fn intentions_info_common() -> Vec<xstaking::IntentionInfoCommon<AccountId, Balance, AuthorityId, BlockNumber >> {
            XStaking::intentions_info_common()
        }
        fn intention_info_common_of(who: &AccountId) -> Option<xstaking::IntentionInfoCommon<AccountId, Balance, AuthorityId, BlockNumber>> {
            XStaking::intention_info_common_of(who)
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
                if let Some((_, info)) = Self::trustee_session_info_for(*chain, None) {
                    map.insert(*chain, info);
                }
            }
            map
        }
        fn trustee_session_info_for(chain: xassets::Chain, number: Option<u32>) -> Option<(u32, GenericAllSessionInfo<AccountId>)> {
            XBridgeFeatures::trustee_session_info_for(chain, number).map(|info| {
                let num = number.unwrap_or(XBridgeFeatures::current_session_number(chain));
                (num, info)
            })
        }
    }

    impl runtime_api::xcontracts_api::XContractsApi<Block> for Runtime {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: u64,
            input_data: Vec<u8>,
        ) -> ContractExecResult {
            let exec_result = XContracts::bare_call(
                origin,
                dest.into(),
                value,
                gas_limit,
                input_data,
            );
            match exec_result {
                Ok(v) => ContractExecResult::Success {
                    status: v.status as u16,
                    data: v.data,
                },
                Err(e) => ContractExecResult::Error(e.reason.as_bytes().to_vec()),
            }
        }

        fn get_storage(
            address: AccountId,
            key: [u8; 32],
        ) -> GetStorageResult {
            XContracts::get_storage(address, key).map_err(|rpc_err| {
                use GetStorageError as RpcGetStorageError;
                /// Map the contract error into the RPC layer error.
                match rpc_err {
                    xcontracts::GetStorageError::ContractDoesntExist => RpcGetStorageError::ContractDoesntExist,
                    xcontracts::GetStorageError::IsTombstone => RpcGetStorageError::IsTombstone,
                }
            })
        }

        fn xrc20_call(token: xassets::Token, selector: XRC20Selector, data: Vec<u8>) -> ContractExecResult {
            // this call should not be called in extrinsics
            let pay_gas = AccountId::default();
            let gas_limit = 100 * 100000000;
            // temp issue some balance for a 0x00...0000 accountid
            let _ = XAssets::pcx_make_free_balance_be(&pay_gas, gas_limit);
            let exec_result = XContracts::call_xrc20(token, pay_gas.clone(), gas_limit, selector, data);
            // remove all balance for this accountid
            let _ = XAssets::pcx_make_free_balance_be(&pay_gas, Zero::zero());
            match exec_result {
                Ok(v) => ContractExecResult::Success {
                    status: v.status as u16,
                    data: v.data,
                },
                Err(e) => ContractExecResult::Error(e.reason.as_bytes().to_vec()),
            }
        }
    }
}
