// Copyright 2018 Chainpool.

//! The ChainX runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]

// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;
#[cfg(feature = "std")]
extern crate serde;

#[macro_use]
extern crate sr_io as runtime_io;
#[macro_use]
extern crate srml_support;
#[macro_use]
extern crate sr_primitives as runtime_primitives;
extern crate parity_codec as codec;
#[macro_use]
extern crate parity_codec_derive;
extern crate substrate_primitives;
#[cfg_attr(not(feature = "std"), macro_use)]
extern crate sr_std as rstd;
extern crate srml_consensus as consensus;
extern crate srml_contract as contract;
extern crate srml_balances as balances;
extern crate srml_council as council;
extern crate srml_democracy as democracy;
extern crate srml_executive as executive;
extern crate srml_session as session;
extern crate srml_staking as staking;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;
extern crate srml_treasury as treasury;
#[macro_use]
extern crate sr_version as version;
extern crate chainx_primitives;

#[cfg(feature = "std")]
mod checked_block;

pub use balances::address::Address as RawAddress;
#[cfg(feature = "std")]
pub use checked_block::CheckedBlock;
pub use consensus::Call as ConsensusCall;
pub use runtime_primitives::Permill;

use rstd::prelude::*;
use substrate_primitives::u32_trait::{_2, _4};
use chainx_primitives::{AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, SessionKey, Signature};
use timestamp::Call as TimestampCall;
use chainx_primitives::InherentData;
use runtime_primitives::generic;
use runtime_primitives::traits::{Convert, BlakeTwo256, DigestItem};
use council::{motions as council_motions, voting as council_voting};
use version::{RuntimeVersion, ApiId};
use codec::{Encode, Decode, Input};

pub fn inherent_extrinsics(data: InherentData) -> Vec<UncheckedExtrinsic> {
    let make_inherent = |function| UncheckedExtrinsic {
        signature: Default::default(),
        function,
        index: 0,
    };

    let mut inherent = vec![
        make_inherent(Call::Timestamp(TimestampCall::set(data.timestamp))),
    ];

    if !data.offline_indices.is_empty() {
        inherent.push(make_inherent(
                Call::Consensus(ConsensusCall::note_offline(data.offline_indices))
        ));
    }

    inherent
}

#[cfg(any(feature = "std", test))]
pub use runtime_primitives::BuildStorage;

const INHERENT: ApiId = *b"inherent";
const VALIDATX: ApiId = *b"validatx";

/// The position of the timestamp set extrinsic.
pub const TIMESTAMP_SET_POSITION: u32 = 0;
/// The position of the offline nodes noting extrinsic.
pub const NOTE_OFFLINE_POSITION: u32 = 2;

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: ver_str!("chainx"),
    impl_name: ver_str!("chainpool-chainx"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 0,
    apis: apis_vec!([(INHERENT, 1), (VALIDATX, 1)]),
};

impl system::Trait for Runtime {
    type Origin = Origin;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type Digest = generic::Digest<Log>;
    type AccountId = AccountId;
    type Header = Header;
    type Event = Event;
    type Log = Log;
}

impl balances::Trait for Runtime {
    type Balance = Balance;
    type AccountIndex = AccountIndex;
    type OnFreeBalanceZero = (Staking, Contract);
    type EnsureAccountLiquid = Staking;
    type Event = Event;
}

impl consensus::Trait for Runtime {
    const NOTE_OFFLINE_POSITION: u32 = NOTE_OFFLINE_POSITION;
    type Log = Log;
    type SessionKey = SessionKey;
    type OnOfflineValidator = Staking;
}

impl timestamp::Trait for Runtime {
    const TIMESTAMP_SET_POSITION: u32 = 0;

    type Moment = u64;
}

/// Session key conversion.
pub struct SessionKeyConversion;
impl Convert<AccountId, SessionKey> for SessionKeyConversion {
    fn convert(a: AccountId) -> SessionKey {
        a.0.into()
    }
}

impl session::Trait for Runtime {
    type ConvertAccountIdToSessionKey = SessionKeyConversion;
    type OnSessionChange = Staking;
    type Event = Event;
}

impl treasury::Trait for Runtime {
    type ApproveOrigin = council_motions::EnsureMembers<_4>;
    type RejectOrigin = council_motions::EnsureMembers<_2>;
    type Event = Event;
}

impl staking::Trait for Runtime {
    type OnRewardMinted = Treasury;
    type Event = Event;
}

impl democracy::Trait for Runtime {
    type Proposal = Call;
    type Event = Event;
}

impl council::Trait for Runtime {
    type Event = Event;
}

impl contract::Trait for Runtime {
    type Gas = u64;
    type DetermineContractAddress = contract::SimpleAddressDeterminator<Runtime>;
}

// TODO add voting and motions at here
impl council::voting::Trait for Runtime {
    type Event = Event;
}

impl council::motions::Trait for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
}

impl DigestItem for Log {
    type Hash = Hash;
    type AuthorityId = SessionKey;

    fn as_authorities_change(&self) -> Option<&[Self::AuthorityId]> {
        match self.0 {
            InternalLog::consensus(ref item) => item.as_authorities_change(),
            _ => None,
        }
    }

    fn as_changes_trie_root(&self) -> Option<&Self::Hash> {
        match self.0 {
            InternalLog::system(ref item) => item.as_changes_trie_root(),
            _ => None,
        }
    }
}

construct_runtime!(
	pub enum Runtime with Log(InternalLog: DigestItem<Hash, SessionKey>) {
		System: system::{default, Log(ChangesTrieRoot)},
		Consensus: consensus::{Module, Call, Storage, Config, Log(AuthoritiesChange)},
		Balances: balances,
		Timestamp: timestamp::{Module, Call, Storage, Config},
		Session: session,
		Staking: staking,
		Democracy: democracy,
		Council: council,
		CouncilVoting: council_voting::{Module, Call, Storage, Event<T>},
		CouncilMotions: council_motions::{Module, Call, Storage, Event<T>, Origin},
		Treasury: treasury,
		Contract: contract::{Module, Call, Config},
	}
);

/// The address format for describing accounts.
pub type Address = balances::Address<Runtime>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Index, Call, Signature>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Index, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, Balances, Balances, AllModules>;

pub mod api {
    impl_stubs!(
		version => |()| super::VERSION,
		authorities => |()| super::Consensus::authorities(),
		initialise_block => |header| super::Executive::initialise_block(&header),
		apply_extrinsic => |extrinsic| super::Executive::apply_extrinsic(extrinsic),
		execute_block => |block| super::Executive::execute_block(block),
		finalise_block => |()| super::Executive::finalise_block(),
        inherent_extrinsics => |inherent| super::inherent_extrinsics(inherent),
		validator_count => |()| super::Session::validator_count(),
        validators => |()| super::Session::validators(),
        timestamp => |()| super::Timestamp::get(),
		random_seed => |()| super::System::random_seed(),
		account_nonce => |account| super::System::account_nonce(&account),
		lookup_address => |address| super::Balances::lookup_address(address)
	);
}
