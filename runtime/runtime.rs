#![feature(prelude_import)]
#![no_std]
// Copyright 2018 Chainpool.

//! The ChainX runtime. This can be compiled with ``#[no_std]`, ready for Wasm.


// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
#[prelude_import]
use ::std::prelude::v1::*;
#[macro_use]
extern crate std;

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
extern crate sr_std as rstd;
extern crate srml_consensus as consensus;
extern crate srml_contract as contract;
extern crate srml_balances as balances;
extern crate srml_council as council;
extern crate srml_democracy as democracy;
extern crate srml_executive as executive;
extern crate srml_session as session;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;
extern crate srml_treasury as treasury;
extern crate cxrml_support as cxsupport;
extern crate cxrml_staking as staking;
extern crate cxrml_tokenbalances as tokenbalances;
extern crate cxrml_financialrecords as financialrecords;
extern crate cxrml_multisig as multisig;
#[macro_use]
extern crate sr_version as version;
extern crate chainx_primitives;

#[cfg(feature = "std")]
mod checked_block {
























    // TODO add voting and motions at here









    // put end of this marco


    // define tokenbalances module type

    //! Typesafe block interaction.
    use super::{Call, Block, TIMESTAMP_SET_POSITION, NOTE_OFFLINE_POSITION};
    use timestamp::Call as TimestampCall;
    /// Provides a type-safe wrapper around a structurally valid block.
    pub struct CheckedBlock {
        inner: Block,
        file_line: Option<(&'static str, u32)>,
    }
    impl CheckedBlock {
        /// Create a new checked block. Fails if the block is not structurally valid.
        pub fn new(block: Block) -> Result<Self, Block> {
            let has_timestamp =
                block.extrinsics.get(TIMESTAMP_SET_POSITION as
                                         usize).map_or(false,
                                                       |xt|
                                                           {
                                                               !xt.is_signed()
                                                                   &&
                                                                   match xt.function
                                                                       {
                                                                       Call::Timestamp(TimestampCall::set(_))
                                                                       =>
                                                                       true,
                                                                       _ =>
                                                                       false,
                                                                   }
                                                           });
            if !has_timestamp { return Err(block); }
            Ok(CheckedBlock{inner: block, file_line: None,})
        }
        #[doc(hidden)]
        pub fn new_unchecked(block: Block, file: &'static str, line: u32)
         -> Self {
            CheckedBlock{inner: block, file_line: Some((file, line)),}
        }
        /// Extract the timestamp from the block.
        pub fn timestamp(&self) -> ::chainx_primitives::Timestamp {
            let x =
                self.inner.extrinsics.get(TIMESTAMP_SET_POSITION as
                                              usize).and_then(|xt|
                                                                  match xt.function
                                                                      {
                                                                      Call::Timestamp(TimestampCall::set(x))
                                                                      =>
                                                                      Some(x),
                                                                      _ =>
                                                                      None,
                                                                  });
            match x {
                Some(x) => x,
                None => {
                    ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Invalid chainx block asserted at "],
                                                                                   &match (&self.file_line,)
                                                                                        {
                                                                                        (arg0,)
                                                                                        =>
                                                                                        [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                     ::std::fmt::Debug::fmt)],
                                                                                    },
                                                                                   &[::std::fmt::rt::v1::Argument{position:
                                                                                                                      ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                  format:
                                                                                                                      ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                         ' ',
                                                                                                                                                     align:
                                                                                                                                                         ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                     flags:
                                                                                                                                                         0u32,
                                                                                                                                                     precision:
                                                                                                                                                         ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                     width:
                                                                                                                                                         ::std::fmt::rt::v1::Count::Implied,},}]),
                                          &("runtime/src/checked_block.rs",
                                            60u32, 21u32))
                }
            }
        }
        /// Extract the noted offline validator indices (if any) from the block.
        pub fn noted_offline(&self) -> &[u32] {
            self.inner.extrinsics.get(NOTE_OFFLINE_POSITION as
                                          usize).and_then(|xt|
                                                              match xt.function
                                                                  {
                                                                  _ => None,
                                                              }).unwrap_or(&[])
        }
        /// Convert into inner block.
        pub fn into_inner(self) -> Block { self.inner }
    }
    impl ::std::ops::Deref for CheckedBlock {
        type
        Target
        =
        Block;
        fn deref(&self) -> &Block { &self.inner }
    }
    /// Assert that a block is structurally valid. May lead to panic in the future
    /// in case it isn't.
    #[macro_export]
    macro_rules! assert_chainx_block(( $ block : expr ) => {
                                     $ crate :: CheckedBlock :: new_unchecked
                                     ( $ block , file ! (  ) , line ! (  ) )
                                     });
}
pub use balances::address::Address as RawAddress;
#[cfg(feature = "std")]
pub use checked_block::CheckedBlock;
pub use consensus::Call as ConsensusCall;
pub use runtime_primitives::{Permill, Perbill};
use rstd::prelude::*;
use substrate_primitives::u32_trait::{_2, _4};
use chainx_primitives::{AccountId, AccountIndex, Balance, BlockNumber, Hash,
                        Index, SessionKey, Signature};
use timestamp::Call as TimestampCall;
use chainx_primitives::InherentData;
use runtime_primitives::generic;
use runtime_primitives::traits::{Convert, BlakeTwo256, DigestItem};
use council::{motions as council_motions, voting as council_voting};
use version::{RuntimeVersion, ApiId};
#[cfg(feature = "std")]
pub use multisig::BalancesConfigCopy;
pub fn inherent_extrinsics(data: InherentData) -> Vec<UncheckedExtrinsic> {
    let mut inherent =
        <[_]>::into_vec(box
                            [generic::UncheckedMortalExtrinsic::new_unsigned(Call::Timestamp(TimestampCall::set(data.timestamp)))]);
    if !data.offline_indices.is_empty() {
        inherent.push(generic::UncheckedMortalExtrinsic::new_unsigned(Call::Consensus(ConsensusCall::note_offline(data.offline_indices))));
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
pub const VERSION: RuntimeVersion =
    RuntimeVersion{spec_name: { ::std::borrow::Cow::Borrowed("chainx") },
                   impl_name:
                       { ::std::borrow::Cow::Borrowed("chainpool-chainx") },
                   authoring_version: 1,
                   spec_version: 1,
                   impl_version: 0,
                   apis:
                       {
                           ::std::borrow::Cow::Borrowed(&[(INHERENT, 1),
                                                          (VALIDATX, 1)])
                       },};
impl system::Trait for Runtime {
    type
    Origin
    =
    Origin;
    type
    Index
    =
    Index;
    type
    BlockNumber
    =
    BlockNumber;
    type
    Hash
    =
    Hash;
    type
    Hashing
    =
    BlakeTwo256;
    type
    Digest
    =
    generic::Digest<Log>;
    type
    AccountId
    =
    AccountId;
    type
    Header
    =
    Header;
    type
    Event
    =
    Event;
    type
    Log
    =
    Log;
}
impl balances::Trait for Runtime {
    type
    Balance
    =
    Balance;
    type
    AccountIndex
    =
    AccountIndex;
    type
    OnFreeBalanceZero
    =
    (Staking, Contract);
    type
    EnsureAccountLiquid
    =
    Staking;
    type
    Event
    =
    Event;
}
impl consensus::Trait for Runtime {
    const
    NOTE_OFFLINE_POSITION:
    u32
    =
    NOTE_OFFLINE_POSITION;
    type
    Log
    =
    Log;
    type
    SessionKey
    =
    SessionKey;
    type
    OnOfflineValidator
    =
    Staking;
}
impl timestamp::Trait for Runtime {
    const
    TIMESTAMP_SET_POSITION:
    u32
    =
    0;
    type
    Moment
    =
    u64;
}
/// Session key conversion.
pub struct SessionKeyConversion;
impl Convert<AccountId, SessionKey> for SessionKeyConversion {
    fn convert(a: AccountId) -> SessionKey { a.0.into() }
}
impl session::Trait for Runtime {
    type
    ConvertAccountIdToSessionKey
    =
    SessionKeyConversion;
    type
    OnSessionChange
    =
    Staking;
    type
    Event
    =
    Event;
}
impl treasury::Trait for Runtime {
    type
    ApproveOrigin
    =
    council_motions::EnsureMembers<_4>;
    type
    RejectOrigin
    =
    council_motions::EnsureMembers<_2>;
    type
    Event
    =
    Event;
}
impl staking::Trait for Runtime {
    type
    OnRewardMinted
    =
    Treasury;
    type
    Event
    =
    Event;
}
impl democracy::Trait for Runtime {
    type
    Proposal
    =
    Call;
    type
    Event
    =
    Event;
}
impl council::Trait for Runtime {
    type
    Event
    =
    Event;
}
impl contract::Trait for Runtime {
    type
    DetermineContractAddress
    =
    contract::SimpleAddressDeterminator<Runtime>;
    type
    Gas
    =
    u64;
}
impl council::voting::Trait for Runtime {
    type
    Event
    =
    Event;
}
impl council::motions::Trait for Runtime {
    type
    Origin
    =
    Origin;
    type
    Proposal
    =
    Call;
    type
    Event
    =
    Event;
}
impl tokenbalances::Trait for Runtime {
    type
    TokenBalance
    =
    TokenBalance;
    type
    Precision
    =
    Precision;
    type
    TokenDesc
    =
    TokenDesc;
    type
    Symbol
    =
    Symbol;
    type
    Event
    =
    Event;
}
impl financialrecords::Trait for Runtime {
    type
    Event
    =
    Event;
}
impl multisig::Trait for Runtime {
    type
    MultiSig
    =
    multisig::SimpleMultiSigIdFor<Runtime>;
    type
    Event
    =
    Event;
}
impl cxsupport::Trait for Runtime { }
impl DigestItem for Log {
    type
    Hash
    =
    Hash;
    type
    AuthorityId
    =
    SessionKey;
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
#[structural_match]
#[rustc_copy_clone_marker]
pub struct Runtime;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::clone::Clone for Runtime {
    #[inline]
    fn clone(&self) -> Runtime { { *self } }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::marker::Copy for Runtime { }
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::cmp::PartialEq for Runtime {
    #[inline]
    fn eq(&self, other: &Runtime) -> bool {
        match *other { Runtime => match *self { Runtime => true, }, }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::cmp::Eq for Runtime {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () { { } }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Runtime => {
                let mut debug_trait_builder = f.debug_tuple("Runtime");
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_Runtime: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl _serde::Serialize for Runtime {
            fn serialize<__S>(&self, __serializer: __S)
             -> _serde::export::Result<__S::Ok, __S::Error> where
             __S: _serde::Serializer {
                _serde::Serializer::serialize_unit_struct(__serializer,
                                                          "Runtime")
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_Runtime: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl <'de> _serde::Deserialize<'de> for Runtime {
            fn deserialize<__D>(__deserializer: __D)
             -> _serde::export::Result<Self, __D::Error> where
             __D: _serde::Deserializer<'de> {
                struct __Visitor;
                impl <'de> _serde::de::Visitor<'de> for __Visitor {
                    type
                    Value
                    =
                    Runtime;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "unit struct Runtime")
                    }
                    #[inline]
                    fn visit_unit<__E>(self)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        _serde::export::Ok(Runtime)
                    }
                }
                _serde::Deserializer::deserialize_unit_struct(__deserializer,
                                                              "Runtime",
                                                              __Visitor)
            }
        }
    };
#[allow(non_camel_case_types)]
#[structural_match]
pub enum Event {
    system(system::Event),
    balances(balances::Event<Runtime>),
    session(session::Event<Runtime>),
    staking(staking::Event<Runtime>),
    democracy(democracy::Event<Runtime>),
    council(council::Event<Runtime>),
    council_voting(council_voting::Event<Runtime>),
    council_motions(council_motions::Event<Runtime>),
    treasury(treasury::Event<Runtime>),
    tokenbalances(tokenbalances::Event<Runtime>),
    financialrecords(financialrecords::Event<Runtime>),
    multisig(multisig::Event<Runtime>),
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::clone::Clone for Event {
    #[inline]
    fn clone(&self) -> Event {
        match (&*self,) {
            (&Event::system(ref __self_0),) =>
            Event::system(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::balances(ref __self_0),) =>
            Event::balances(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::session(ref __self_0),) =>
            Event::session(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::staking(ref __self_0),) =>
            Event::staking(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::democracy(ref __self_0),) =>
            Event::democracy(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::council(ref __self_0),) =>
            Event::council(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::council_voting(ref __self_0),) =>
            Event::council_voting(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::council_motions(ref __self_0),) =>
            Event::council_motions(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::treasury(ref __self_0),) =>
            Event::treasury(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::tokenbalances(ref __self_0),) =>
            Event::tokenbalances(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::financialrecords(ref __self_0),) =>
            Event::financialrecords(::std::clone::Clone::clone(&(*__self_0))),
            (&Event::multisig(ref __self_0),) =>
            Event::multisig(::std::clone::Clone::clone(&(*__self_0))),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::PartialEq for Event {
    #[inline]
    fn eq(&self, other: &Event) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Event::system(ref __self_0),
                     &Event::system(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::balances(ref __self_0),
                     &Event::balances(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::session(ref __self_0),
                     &Event::session(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::staking(ref __self_0),
                     &Event::staking(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::democracy(ref __self_0),
                     &Event::democracy(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::council(ref __self_0),
                     &Event::council(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::council_voting(ref __self_0),
                     &Event::council_voting(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::council_motions(ref __self_0),
                     &Event::council_motions(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::treasury(ref __self_0),
                     &Event::treasury(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::tokenbalances(ref __self_0),
                     &Event::tokenbalances(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::financialrecords(ref __self_0),
                     &Event::financialrecords(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Event::multisig(ref __self_0),
                     &Event::multisig(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { false }
        }
    }
    #[inline]
    fn ne(&self, other: &Event) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Event::system(ref __self_0),
                     &Event::system(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::balances(ref __self_0),
                     &Event::balances(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::session(ref __self_0),
                     &Event::session(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::staking(ref __self_0),
                     &Event::staking(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::democracy(ref __self_0),
                     &Event::democracy(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::council(ref __self_0),
                     &Event::council(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::council_voting(ref __self_0),
                     &Event::council_voting(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::council_motions(ref __self_0),
                     &Event::council_motions(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::treasury(ref __self_0),
                     &Event::treasury(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::tokenbalances(ref __self_0),
                     &Event::tokenbalances(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::financialrecords(ref __self_0),
                     &Event::financialrecords(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Event::multisig(ref __self_0),
                     &Event::multisig(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { true }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::Eq for Event {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::std::cmp::AssertParamIsEq<system::Event>;
            let _: ::std::cmp::AssertParamIsEq<balances::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<session::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<staking::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<democracy::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<council::Event<Runtime>>;
            let _:
                    ::std::cmp::AssertParamIsEq<council_voting::Event<Runtime>>;
            let _:
                    ::std::cmp::AssertParamIsEq<council_motions::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<treasury::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<tokenbalances::Event<Runtime>>;
            let _:
                    ::std::cmp::AssertParamIsEq<financialrecords::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<multisig::Event<Runtime>>;
        }
    }
}
impl ::codec::Encode for Event {
    fn encode_to<EncOut: ::codec::Output>(&self, dest: &mut EncOut) {
        match *self {
            Event::system(ref aa) => {
                dest.push_byte(0usize as u8);
                dest.push(aa);
            }
            Event::balances(ref aa) => {
                dest.push_byte(1usize as u8);
                dest.push(aa);
            }
            Event::session(ref aa) => {
                dest.push_byte(2usize as u8);
                dest.push(aa);
            }
            Event::staking(ref aa) => {
                dest.push_byte(3usize as u8);
                dest.push(aa);
            }
            Event::democracy(ref aa) => {
                dest.push_byte(4usize as u8);
                dest.push(aa);
            }
            Event::council(ref aa) => {
                dest.push_byte(5usize as u8);
                dest.push(aa);
            }
            Event::council_voting(ref aa) => {
                dest.push_byte(6usize as u8);
                dest.push(aa);
            }
            Event::council_motions(ref aa) => {
                dest.push_byte(7usize as u8);
                dest.push(aa);
            }
            Event::treasury(ref aa) => {
                dest.push_byte(8usize as u8);
                dest.push(aa);
            }
            Event::tokenbalances(ref aa) => {
                dest.push_byte(9usize as u8);
                dest.push(aa);
            }
            Event::financialrecords(ref aa) => {
                dest.push_byte(10usize as u8);
                dest.push(aa);
            }
            Event::multisig(ref aa) => {
                dest.push_byte(11usize as u8);
                dest.push(aa);
            }
        }
    }
}
impl ::codec::Decode for Event {
    fn decode<DecIn: ::codec::Input>(input: &mut DecIn) -> Option<Self> {
        match input.read_byte()? {
            x if x == 0usize as u8 => {
                Some(Event::system(::codec::Decode::decode(input)?))
            }
            x if x == 1usize as u8 => {
                Some(Event::balances(::codec::Decode::decode(input)?))
            }
            x if x == 2usize as u8 => {
                Some(Event::session(::codec::Decode::decode(input)?))
            }
            x if x == 3usize as u8 => {
                Some(Event::staking(::codec::Decode::decode(input)?))
            }
            x if x == 4usize as u8 => {
                Some(Event::democracy(::codec::Decode::decode(input)?))
            }
            x if x == 5usize as u8 => {
                Some(Event::council(::codec::Decode::decode(input)?))
            }
            x if x == 6usize as u8 => {
                Some(Event::council_voting(::codec::Decode::decode(input)?))
            }
            x if x == 7usize as u8 => {
                Some(Event::council_motions(::codec::Decode::decode(input)?))
            }
            x if x == 8usize as u8 => {
                Some(Event::treasury(::codec::Decode::decode(input)?))
            }
            x if x == 9usize as u8 => {
                Some(Event::tokenbalances(::codec::Decode::decode(input)?))
            }
            x if x == 10usize as u8 => {
                Some(Event::financialrecords(::codec::Decode::decode(input)?))
            }
            x if x == 11usize as u8 => {
                Some(Event::multisig(::codec::Decode::decode(input)?))
            }
            _ => None,
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::fmt::Debug for Event {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match (&*self,) {
            (&Event::system(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("system");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::balances(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("balances");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::session(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("session");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::staking(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("staking");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::democracy(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("democracy");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::council(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("council");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::council_voting(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("council_voting");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::council_motions(ref __self_0),) => {
                let mut debug_trait_builder =
                    f.debug_tuple("council_motions");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::treasury(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("treasury");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::tokenbalances(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("tokenbalances");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::financialrecords(ref __self_0),) => {
                let mut debug_trait_builder =
                    f.debug_tuple("financialrecords");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::multisig(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("multisig");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_Event: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl _serde::Serialize for Event {
            fn serialize<__S>(&self, __serializer: __S)
             -> _serde::export::Result<__S::Ok, __S::Error> where
             __S: _serde::Serializer {
                match *self {
                    Event::system(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  0u32,
                                                                  "system",
                                                                  __field0),
                    Event::balances(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  1u32,
                                                                  "balances",
                                                                  __field0),
                    Event::session(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  2u32,
                                                                  "session",
                                                                  __field0),
                    Event::staking(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  3u32,
                                                                  "staking",
                                                                  __field0),
                    Event::democracy(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  4u32,
                                                                  "democracy",
                                                                  __field0),
                    Event::council(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  5u32,
                                                                  "council",
                                                                  __field0),
                    Event::council_voting(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  6u32,
                                                                  "council_voting",
                                                                  __field0),
                    Event::council_motions(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  7u32,
                                                                  "council_motions",
                                                                  __field0),
                    Event::treasury(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  8u32,
                                                                  "treasury",
                                                                  __field0),
                    Event::tokenbalances(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  9u32,
                                                                  "tokenbalances",
                                                                  __field0),
                    Event::financialrecords(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  10u32,
                                                                  "financialrecords",
                                                                  __field0),
                    Event::multisig(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Event",
                                                                  11u32,
                                                                  "multisig",
                                                                  __field0),
                }
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_Event: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl <'de> _serde::Deserialize<'de> for Event {
            fn deserialize<__D>(__deserializer: __D)
             -> _serde::export::Result<Self, __D::Error> where
             __D: _serde::Deserializer<'de> {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __field3,
                    __field4,
                    __field5,
                    __field6,
                    __field7,
                    __field8,
                    __field9,
                    __field10,
                    __field11,
                }
                struct __FieldVisitor;
                impl <'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type
                    Value
                    =
                    __Field;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "variant identifier")
                    }
                    fn visit_u64<__E>(self, __value: u64)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            0u64 => _serde::export::Ok(__Field::__field0),
                            1u64 => _serde::export::Ok(__Field::__field1),
                            2u64 => _serde::export::Ok(__Field::__field2),
                            3u64 => _serde::export::Ok(__Field::__field3),
                            4u64 => _serde::export::Ok(__Field::__field4),
                            5u64 => _serde::export::Ok(__Field::__field5),
                            6u64 => _serde::export::Ok(__Field::__field6),
                            7u64 => _serde::export::Ok(__Field::__field7),
                            8u64 => _serde::export::Ok(__Field::__field8),
                            9u64 => _serde::export::Ok(__Field::__field9),
                            10u64 => _serde::export::Ok(__Field::__field10),
                            11u64 => _serde::export::Ok(__Field::__field11),
                            _ =>
                            _serde::export::Err(_serde::de::Error::invalid_value(_serde::de::Unexpected::Unsigned(__value),
                                                                                 &"variant index 0 <= i < 12")),
                        }
                    }
                    fn visit_str<__E>(self, __value: &str)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            "system" => _serde::export::Ok(__Field::__field0),
                            "balances" =>
                            _serde::export::Ok(__Field::__field1),
                            "session" =>
                            _serde::export::Ok(__Field::__field2),
                            "staking" =>
                            _serde::export::Ok(__Field::__field3),
                            "democracy" =>
                            _serde::export::Ok(__Field::__field4),
                            "council" =>
                            _serde::export::Ok(__Field::__field5),
                            "council_voting" =>
                            _serde::export::Ok(__Field::__field6),
                            "council_motions" =>
                            _serde::export::Ok(__Field::__field7),
                            "treasury" =>
                            _serde::export::Ok(__Field::__field8),
                            "tokenbalances" =>
                            _serde::export::Ok(__Field::__field9),
                            "financialrecords" =>
                            _serde::export::Ok(__Field::__field10),
                            "multisig" =>
                            _serde::export::Ok(__Field::__field11),
                            _ => {
                                _serde::export::Err(_serde::de::Error::unknown_variant(__value,
                                                                                       VARIANTS))
                            }
                        }
                    }
                    fn visit_bytes<__E>(self, __value: &[u8])
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            b"system" =>
                            _serde::export::Ok(__Field::__field0),
                            b"balances" =>
                            _serde::export::Ok(__Field::__field1),
                            b"session" =>
                            _serde::export::Ok(__Field::__field2),
                            b"staking" =>
                            _serde::export::Ok(__Field::__field3),
                            b"democracy" =>
                            _serde::export::Ok(__Field::__field4),
                            b"council" =>
                            _serde::export::Ok(__Field::__field5),
                            b"council_voting" =>
                            _serde::export::Ok(__Field::__field6),
                            b"council_motions" =>
                            _serde::export::Ok(__Field::__field7),
                            b"treasury" =>
                            _serde::export::Ok(__Field::__field8),
                            b"tokenbalances" =>
                            _serde::export::Ok(__Field::__field9),
                            b"financialrecords" =>
                            _serde::export::Ok(__Field::__field10),
                            b"multisig" =>
                            _serde::export::Ok(__Field::__field11),
                            _ => {
                                let __value =
                                    &_serde::export::from_utf8_lossy(__value);
                                _serde::export::Err(_serde::de::Error::unknown_variant(__value,
                                                                                       VARIANTS))
                            }
                        }
                    }
                }
                impl <'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::export::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                     __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::export::PhantomData<Event>,
                    lifetime: _serde::export::PhantomData<&'de ()>,
                }
                impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type
                    Value
                    =
                    Event;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "enum Event")
                    }
                    fn visit_enum<__A>(self, __data: __A)
                     -> _serde::export::Result<Self::Value, __A::Error> where
                     __A: _serde::de::EnumAccess<'de> {
                        match match _serde::de::EnumAccess::variant(__data) {
                                  _serde::export::Ok(__val) => __val,
                                  _serde::export::Err(__err) => {
                                      return _serde::export::Err(__err);
                                  }
                              } {
                            (__Field::__field0, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<system::Event>(__variant),
                                                        Event::system),
                            (__Field::__field1, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<balances::Event<Runtime>>(__variant),
                                                        Event::balances),
                            (__Field::__field2, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<session::Event<Runtime>>(__variant),
                                                        Event::session),
                            (__Field::__field3, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<staking::Event<Runtime>>(__variant),
                                                        Event::staking),
                            (__Field::__field4, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<democracy::Event<Runtime>>(__variant),
                                                        Event::democracy),
                            (__Field::__field5, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<council::Event<Runtime>>(__variant),
                                                        Event::council),
                            (__Field::__field6, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<council_voting::Event<Runtime>>(__variant),
                                                        Event::council_voting),
                            (__Field::__field7, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<council_motions::Event<Runtime>>(__variant),
                                                        Event::council_motions),
                            (__Field::__field8, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<treasury::Event<Runtime>>(__variant),
                                                        Event::treasury),
                            (__Field::__field9, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<tokenbalances::Event<Runtime>>(__variant),
                                                        Event::tokenbalances),
                            (__Field::__field10, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<financialrecords::Event<Runtime>>(__variant),
                                                        Event::financialrecords),
                            (__Field::__field11, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<multisig::Event<Runtime>>(__variant),
                                                        Event::multisig),
                        }
                    }
                }
                const VARIANTS: &'static [&'static str] =
                    &["system", "balances", "session", "staking", "democracy",
                      "council", "council_voting", "council_motions",
                      "treasury", "tokenbalances", "financialrecords",
                      "multisig"];
                _serde::Deserializer::deserialize_enum(__deserializer,
                                                       "Event", VARIANTS,
                                                       __Visitor{marker:
                                                                     _serde::export::PhantomData::<Event>,
                                                                 lifetime:
                                                                     _serde::export::PhantomData,})
            }
        }
    };
impl From<system::Event> for Event {
    fn from(x: system::Event) -> Self { Event::system(x) }
}
impl From<balances::Event<Runtime>> for Event {
    fn from(x: balances::Event<Runtime>) -> Self { Event::balances(x) }
}
impl From<session::Event<Runtime>> for Event {
    fn from(x: session::Event<Runtime>) -> Self { Event::session(x) }
}
impl From<staking::Event<Runtime>> for Event {
    fn from(x: staking::Event<Runtime>) -> Self { Event::staking(x) }
}
impl From<democracy::Event<Runtime>> for Event {
    fn from(x: democracy::Event<Runtime>) -> Self { Event::democracy(x) }
}
impl From<council::Event<Runtime>> for Event {
    fn from(x: council::Event<Runtime>) -> Self { Event::council(x) }
}
impl From<council_voting::Event<Runtime>> for Event {
    fn from(x: council_voting::Event<Runtime>) -> Self {
        Event::council_voting(x)
    }
}
impl From<council_motions::Event<Runtime>> for Event {
    fn from(x: council_motions::Event<Runtime>) -> Self {
        Event::council_motions(x)
    }
}
impl From<treasury::Event<Runtime>> for Event {
    fn from(x: treasury::Event<Runtime>) -> Self { Event::treasury(x) }
}
impl From<tokenbalances::Event<Runtime>> for Event {
    fn from(x: tokenbalances::Event<Runtime>) -> Self {
        Event::tokenbalances(x)
    }
}
impl From<financialrecords::Event<Runtime>> for Event {
    fn from(x: financialrecords::Event<Runtime>) -> Self {
        Event::financialrecords(x)
    }
}
impl From<multisig::Event<Runtime>> for Event {
    fn from(x: multisig::Event<Runtime>) -> Self { Event::multisig(x) }
}
impl Runtime {
    #[allow(dead_code)]
    pub fn outer_event_metadata() -> ::event::OuterEventMetadata {
        ::event::OuterEventMetadata{name:
                                        ::event::DecodeDifferent::Encode("Event"),
                                    events:
                                        ::event::DecodeDifferent::Encode(&[("system",
                                                                            ::event::FnEncode(system::Event::metadata)),
                                                                           ("balances",
                                                                            ::event::FnEncode(balances::Event::<Runtime>::metadata)),
                                                                           ("session",
                                                                            ::event::FnEncode(session::Event::<Runtime>::metadata)),
                                                                           ("staking",
                                                                            ::event::FnEncode(staking::Event::<Runtime>::metadata)),
                                                                           ("democracy",
                                                                            ::event::FnEncode(democracy::Event::<Runtime>::metadata)),
                                                                           ("council",
                                                                            ::event::FnEncode(council::Event::<Runtime>::metadata)),
                                                                           ("council_voting",
                                                                            ::event::FnEncode(council_voting::Event::<Runtime>::metadata)),
                                                                           ("council_motions",
                                                                            ::event::FnEncode(council_motions::Event::<Runtime>::metadata)),
                                                                           ("treasury",
                                                                            ::event::FnEncode(treasury::Event::<Runtime>::metadata)),
                                                                           ("tokenbalances",
                                                                            ::event::FnEncode(tokenbalances::Event::<Runtime>::metadata)),
                                                                           ("financialrecords",
                                                                            ::event::FnEncode(financialrecords::Event::<Runtime>::metadata)),
                                                                           ("multisig",
                                                                            ::event::FnEncode(multisig::Event::<Runtime>::metadata))]),}
    }
}
#[allow(non_camel_case_types)]
#[structural_match]
pub enum Origin {
    system(system::Origin<Runtime>),
    council_motions(council_motions::Origin),

    #[allow(dead_code)]
    Void(::Void),
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::clone::Clone for Origin {
    #[inline]
    fn clone(&self) -> Origin {
        match (&*self,) {
            (&Origin::system(ref __self_0),) =>
            Origin::system(::std::clone::Clone::clone(&(*__self_0))),
            (&Origin::council_motions(ref __self_0),) =>
            Origin::council_motions(::std::clone::Clone::clone(&(*__self_0))),
            (&Origin::Void(ref __self_0),) =>
            Origin::Void(::std::clone::Clone::clone(&(*__self_0))),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::PartialEq for Origin {
    #[inline]
    fn eq(&self, other: &Origin) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Origin::system(ref __self_0),
                     &Origin::system(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Origin::council_motions(ref __self_0),
                     &Origin::council_motions(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Origin::Void(ref __self_0),
                     &Origin::Void(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { false }
        }
    }
    #[inline]
    fn ne(&self, other: &Origin) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Origin::system(ref __self_0),
                     &Origin::system(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Origin::council_motions(ref __self_0),
                     &Origin::council_motions(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Origin::Void(ref __self_0),
                     &Origin::Void(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { true }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::Eq for Origin {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::std::cmp::AssertParamIsEq<system::Origin<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<council_motions::Origin>;
            let _: ::std::cmp::AssertParamIsEq<::Void>;
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::fmt::Debug for Origin {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match (&*self,) {
            (&Origin::system(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("system");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Origin::council_motions(ref __self_0),) => {
                let mut debug_trait_builder =
                    f.debug_tuple("council_motions");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Origin::Void(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Void");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(dead_code)]
impl Origin {
    pub const
    INHERENT:
    Self
    =
    Origin::system(system::RawOrigin::Inherent);
    pub const
    ROOT:
    Self
    =
    Origin::system(system::RawOrigin::Root);
    pub fn signed(by: <Runtime as system::Trait>::AccountId) -> Self {
        Origin::system(system::RawOrigin::Signed(by))
    }
}
impl From<system::Origin<Runtime>> for Origin {
    fn from(x: system::Origin<Runtime>) -> Self { Origin::system(x) }
}
impl Into<Option<system::Origin<Runtime>>> for Origin {
    fn into(self) -> Option<system::Origin<Runtime>> {
        if let Origin::system(l) = self { Some(l) } else { None }
    }
}
impl From<Option<<Runtime as system::Trait>::AccountId>> for Origin {
    fn from(x: Option<<Runtime as system::Trait>::AccountId>) -> Self {
        <system::Origin<Runtime>>::from(x).into()
    }
}
impl From<council_motions::Origin> for Origin {
    fn from(x: council_motions::Origin) -> Self { Origin::council_motions(x) }
}
impl Into<Option<council_motions::Origin>> for Origin {
    fn into(self) -> Option<council_motions::Origin> {
        if let Origin::council_motions(l) = self { Some(l) } else { None }
    }
}
pub type System = system::Module<Runtime>;
pub type Consensus = consensus::Module<Runtime>;
pub type Balances = balances::Module<Runtime>;
pub type Timestamp = timestamp::Module<Runtime>;
pub type Session = session::Module<Runtime>;
pub type Staking = staking::Module<Runtime>;
pub type Democracy = democracy::Module<Runtime>;
pub type Council = council::Module<Runtime>;
pub type CouncilVoting = council_voting::Module<Runtime>;
pub type CouncilMotions = council_motions::Module<Runtime>;
pub type Treasury = treasury::Module<Runtime>;
pub type Contract = contract::Module<Runtime>;
pub type TokenBalances = tokenbalances::Module<Runtime>;
pub type FinancialRecords = financialrecords::Module<Runtime>;
pub type MultiSig = multisig::Module<Runtime>;
pub type CXSupport = cxsupport::Module<Runtime>;
type AllModules
    =
    (Consensus, Balances, Timestamp, Session, Staking, Democracy, Council,
     CouncilVoting, CouncilMotions, Treasury, Contract, TokenBalances,
     FinancialRecords, MultiSig, CXSupport);
#[structural_match]
pub enum Call {
    Consensus(::dispatch::CallableCallFor<Consensus>),
    Balances(::dispatch::CallableCallFor<Balances>),
    Timestamp(::dispatch::CallableCallFor<Timestamp>),
    Session(::dispatch::CallableCallFor<Session>),
    Staking(::dispatch::CallableCallFor<Staking>),
    Democracy(::dispatch::CallableCallFor<Democracy>),
    Council(::dispatch::CallableCallFor<Council>),
    CouncilVoting(::dispatch::CallableCallFor<CouncilVoting>),
    CouncilMotions(::dispatch::CallableCallFor<CouncilMotions>),
    Treasury(::dispatch::CallableCallFor<Treasury>),
    Contract(::dispatch::CallableCallFor<Contract>),
    TokenBalances(::dispatch::CallableCallFor<TokenBalances>),
    FinancialRecords(::dispatch::CallableCallFor<FinancialRecords>),
    MultiSig(::dispatch::CallableCallFor<MultiSig>),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::clone::Clone for Call {
    #[inline]
    fn clone(&self) -> Call {
        match (&*self,) {
            (&Call::Consensus(ref __self_0),) =>
            Call::Consensus(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Balances(ref __self_0),) =>
            Call::Balances(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Timestamp(ref __self_0),) =>
            Call::Timestamp(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Session(ref __self_0),) =>
            Call::Session(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Staking(ref __self_0),) =>
            Call::Staking(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Democracy(ref __self_0),) =>
            Call::Democracy(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Council(ref __self_0),) =>
            Call::Council(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::CouncilVoting(ref __self_0),) =>
            Call::CouncilVoting(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::CouncilMotions(ref __self_0),) =>
            Call::CouncilMotions(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Treasury(ref __self_0),) =>
            Call::Treasury(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::Contract(ref __self_0),) =>
            Call::Contract(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::TokenBalances(ref __self_0),) =>
            Call::TokenBalances(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::FinancialRecords(ref __self_0),) =>
            Call::FinancialRecords(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::MultiSig(ref __self_0),) =>
            Call::MultiSig(::std::clone::Clone::clone(&(*__self_0))),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::cmp::PartialEq for Call {
    #[inline]
    fn eq(&self, other: &Call) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Call::Consensus(ref __self_0),
                     &Call::Consensus(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Balances(ref __self_0),
                     &Call::Balances(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Timestamp(ref __self_0),
                     &Call::Timestamp(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Session(ref __self_0),
                     &Call::Session(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Staking(ref __self_0),
                     &Call::Staking(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Democracy(ref __self_0),
                     &Call::Democracy(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Council(ref __self_0),
                     &Call::Council(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::CouncilVoting(ref __self_0),
                     &Call::CouncilVoting(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::CouncilMotions(ref __self_0),
                     &Call::CouncilMotions(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Treasury(ref __self_0),
                     &Call::Treasury(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::Contract(ref __self_0),
                     &Call::Contract(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::TokenBalances(ref __self_0),
                     &Call::TokenBalances(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::FinancialRecords(ref __self_0),
                     &Call::FinancialRecords(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&Call::MultiSig(ref __self_0),
                     &Call::MultiSig(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { false }
        }
    }
    #[inline]
    fn ne(&self, other: &Call) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Call::Consensus(ref __self_0),
                     &Call::Consensus(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Balances(ref __self_0),
                     &Call::Balances(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Timestamp(ref __self_0),
                     &Call::Timestamp(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Session(ref __self_0),
                     &Call::Session(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Staking(ref __self_0),
                     &Call::Staking(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Democracy(ref __self_0),
                     &Call::Democracy(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Council(ref __self_0),
                     &Call::Council(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::CouncilVoting(ref __self_0),
                     &Call::CouncilVoting(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::CouncilMotions(ref __self_0),
                     &Call::CouncilMotions(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Treasury(ref __self_0),
                     &Call::Treasury(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::Contract(ref __self_0),
                     &Call::Contract(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::TokenBalances(ref __self_0),
                     &Call::TokenBalances(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::FinancialRecords(ref __self_0),
                     &Call::FinancialRecords(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&Call::MultiSig(ref __self_0),
                     &Call::MultiSig(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { true }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::cmp::Eq for Call {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Consensus>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Balances>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Timestamp>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Session>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Staking>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Democracy>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Council>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<CouncilVoting>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<CouncilMotions>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Treasury>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<Contract>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<TokenBalances>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<FinancialRecords>>;
            let _:
                    ::std::cmp::AssertParamIsEq<::dispatch::CallableCallFor<MultiSig>>;
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::fmt::Debug for Call {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match (&*self,) {
            (&Call::Consensus(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Consensus");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Balances(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Balances");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Timestamp(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Timestamp");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Session(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Session");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Staking(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Staking");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Democracy(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Democracy");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Council(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Council");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::CouncilVoting(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("CouncilVoting");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::CouncilMotions(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("CouncilMotions");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Treasury(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Treasury");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Contract(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Contract");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::TokenBalances(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("TokenBalances");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::FinancialRecords(ref __self_0),) => {
                let mut debug_trait_builder =
                    f.debug_tuple("FinancialRecords");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::MultiSig(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("MultiSig");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_Call: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl _serde::Serialize for Call {
            fn serialize<__S>(&self, __serializer: __S)
             -> _serde::export::Result<__S::Ok, __S::Error> where
             __S: _serde::Serializer {
                match *self {
                    Call::Consensus(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  0u32,
                                                                  "Consensus",
                                                                  __field0),
                    Call::Balances(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  1u32,
                                                                  "Balances",
                                                                  __field0),
                    Call::Timestamp(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  2u32,
                                                                  "Timestamp",
                                                                  __field0),
                    Call::Session(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  3u32,
                                                                  "Session",
                                                                  __field0),
                    Call::Staking(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  4u32,
                                                                  "Staking",
                                                                  __field0),
                    Call::Democracy(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  5u32,
                                                                  "Democracy",
                                                                  __field0),
                    Call::Council(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  6u32,
                                                                  "Council",
                                                                  __field0),
                    Call::CouncilVoting(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  7u32,
                                                                  "CouncilVoting",
                                                                  __field0),
                    Call::CouncilMotions(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  8u32,
                                                                  "CouncilMotions",
                                                                  __field0),
                    Call::Treasury(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  9u32,
                                                                  "Treasury",
                                                                  __field0),
                    Call::Contract(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  10u32,
                                                                  "Contract",
                                                                  __field0),
                    Call::TokenBalances(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  11u32,
                                                                  "TokenBalances",
                                                                  __field0),
                    Call::FinancialRecords(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  12u32,
                                                                  "FinancialRecords",
                                                                  __field0),
                    Call::MultiSig(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "Call",
                                                                  13u32,
                                                                  "MultiSig",
                                                                  __field0),
                }
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_Call: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl <'de> _serde::Deserialize<'de> for Call {
            fn deserialize<__D>(__deserializer: __D)
             -> _serde::export::Result<Self, __D::Error> where
             __D: _serde::Deserializer<'de> {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __field3,
                    __field4,
                    __field5,
                    __field6,
                    __field7,
                    __field8,
                    __field9,
                    __field10,
                    __field11,
                    __field12,
                    __field13,
                }
                struct __FieldVisitor;
                impl <'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type
                    Value
                    =
                    __Field;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "variant identifier")
                    }
                    fn visit_u64<__E>(self, __value: u64)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            0u64 => _serde::export::Ok(__Field::__field0),
                            1u64 => _serde::export::Ok(__Field::__field1),
                            2u64 => _serde::export::Ok(__Field::__field2),
                            3u64 => _serde::export::Ok(__Field::__field3),
                            4u64 => _serde::export::Ok(__Field::__field4),
                            5u64 => _serde::export::Ok(__Field::__field5),
                            6u64 => _serde::export::Ok(__Field::__field6),
                            7u64 => _serde::export::Ok(__Field::__field7),
                            8u64 => _serde::export::Ok(__Field::__field8),
                            9u64 => _serde::export::Ok(__Field::__field9),
                            10u64 => _serde::export::Ok(__Field::__field10),
                            11u64 => _serde::export::Ok(__Field::__field11),
                            12u64 => _serde::export::Ok(__Field::__field12),
                            13u64 => _serde::export::Ok(__Field::__field13),
                            _ =>
                            _serde::export::Err(_serde::de::Error::invalid_value(_serde::de::Unexpected::Unsigned(__value),
                                                                                 &"variant index 0 <= i < 14")),
                        }
                    }
                    fn visit_str<__E>(self, __value: &str)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            "Consensus" =>
                            _serde::export::Ok(__Field::__field0),
                            "Balances" =>
                            _serde::export::Ok(__Field::__field1),
                            "Timestamp" =>
                            _serde::export::Ok(__Field::__field2),
                            "Session" =>
                            _serde::export::Ok(__Field::__field3),
                            "Staking" =>
                            _serde::export::Ok(__Field::__field4),
                            "Democracy" =>
                            _serde::export::Ok(__Field::__field5),
                            "Council" =>
                            _serde::export::Ok(__Field::__field6),
                            "CouncilVoting" =>
                            _serde::export::Ok(__Field::__field7),
                            "CouncilMotions" =>
                            _serde::export::Ok(__Field::__field8),
                            "Treasury" =>
                            _serde::export::Ok(__Field::__field9),
                            "Contract" =>
                            _serde::export::Ok(__Field::__field10),
                            "TokenBalances" =>
                            _serde::export::Ok(__Field::__field11),
                            "FinancialRecords" =>
                            _serde::export::Ok(__Field::__field12),
                            "MultiSig" =>
                            _serde::export::Ok(__Field::__field13),
                            _ => {
                                _serde::export::Err(_serde::de::Error::unknown_variant(__value,
                                                                                       VARIANTS))
                            }
                        }
                    }
                    fn visit_bytes<__E>(self, __value: &[u8])
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            b"Consensus" =>
                            _serde::export::Ok(__Field::__field0),
                            b"Balances" =>
                            _serde::export::Ok(__Field::__field1),
                            b"Timestamp" =>
                            _serde::export::Ok(__Field::__field2),
                            b"Session" =>
                            _serde::export::Ok(__Field::__field3),
                            b"Staking" =>
                            _serde::export::Ok(__Field::__field4),
                            b"Democracy" =>
                            _serde::export::Ok(__Field::__field5),
                            b"Council" =>
                            _serde::export::Ok(__Field::__field6),
                            b"CouncilVoting" =>
                            _serde::export::Ok(__Field::__field7),
                            b"CouncilMotions" =>
                            _serde::export::Ok(__Field::__field8),
                            b"Treasury" =>
                            _serde::export::Ok(__Field::__field9),
                            b"Contract" =>
                            _serde::export::Ok(__Field::__field10),
                            b"TokenBalances" =>
                            _serde::export::Ok(__Field::__field11),
                            b"FinancialRecords" =>
                            _serde::export::Ok(__Field::__field12),
                            b"MultiSig" =>
                            _serde::export::Ok(__Field::__field13),
                            _ => {
                                let __value =
                                    &_serde::export::from_utf8_lossy(__value);
                                _serde::export::Err(_serde::de::Error::unknown_variant(__value,
                                                                                       VARIANTS))
                            }
                        }
                    }
                }
                impl <'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::export::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                     __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::export::PhantomData<Call>,
                    lifetime: _serde::export::PhantomData<&'de ()>,
                }
                impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type
                    Value
                    =
                    Call;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "enum Call")
                    }
                    fn visit_enum<__A>(self, __data: __A)
                     -> _serde::export::Result<Self::Value, __A::Error> where
                     __A: _serde::de::EnumAccess<'de> {
                        match match _serde::de::EnumAccess::variant(__data) {
                                  _serde::export::Ok(__val) => __val,
                                  _serde::export::Err(__err) => {
                                      return _serde::export::Err(__err);
                                  }
                              } {
                            (__Field::__field0, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Consensus>>(__variant),
                                                        Call::Consensus),
                            (__Field::__field1, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Balances>>(__variant),
                                                        Call::Balances),
                            (__Field::__field2, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Timestamp>>(__variant),
                                                        Call::Timestamp),
                            (__Field::__field3, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Session>>(__variant),
                                                        Call::Session),
                            (__Field::__field4, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Staking>>(__variant),
                                                        Call::Staking),
                            (__Field::__field5, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Democracy>>(__variant),
                                                        Call::Democracy),
                            (__Field::__field6, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Council>>(__variant),
                                                        Call::Council),
                            (__Field::__field7, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<CouncilVoting>>(__variant),
                                                        Call::CouncilVoting),
                            (__Field::__field8, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<CouncilMotions>>(__variant),
                                                        Call::CouncilMotions),
                            (__Field::__field9, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Treasury>>(__variant),
                                                        Call::Treasury),
                            (__Field::__field10, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<Contract>>(__variant),
                                                        Call::Contract),
                            (__Field::__field11, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<TokenBalances>>(__variant),
                                                        Call::TokenBalances),
                            (__Field::__field12, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<FinancialRecords>>(__variant),
                                                        Call::FinancialRecords),
                            (__Field::__field13, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<::srml_support::dispatch::CallableCallFor<MultiSig>>(__variant),
                                                        Call::MultiSig),
                        }
                    }
                }
                const VARIANTS: &'static [&'static str] =
                    &["Consensus", "Balances", "Timestamp", "Session",
                      "Staking", "Democracy", "Council", "CouncilVoting",
                      "CouncilMotions", "Treasury", "Contract",
                      "TokenBalances", "FinancialRecords", "MultiSig"];
                _serde::Deserializer::deserialize_enum(__deserializer, "Call",
                                                       VARIANTS,
                                                       __Visitor{marker:
                                                                     _serde::export::PhantomData::<Call>,
                                                                 lifetime:
                                                                     _serde::export::PhantomData,})
            }
        }
    };
impl ::dispatch::Decode for Call {
    fn decode<I: ::dispatch::Input>(input: &mut I) -> Option<Self> {
        let input_id = input.read_byte()?;
        {
            if input_id == (0) {
                let outer_dispatch_param = ::dispatch::Decode::decode(input)?;
                return Some(Call::Consensus(outer_dispatch_param));
            }
            {
                if input_id == (0 + 1) {
                    let outer_dispatch_param =
                        ::dispatch::Decode::decode(input)?;
                    return Some(Call::Balances(outer_dispatch_param));
                }
                {
                    if input_id == (0 + 1 + 1) {
                        let outer_dispatch_param =
                            ::dispatch::Decode::decode(input)?;
                        return Some(Call::Timestamp(outer_dispatch_param));
                    }
                    {
                        if input_id == (0 + 1 + 1 + 1) {
                            let outer_dispatch_param =
                                ::dispatch::Decode::decode(input)?;
                            return Some(Call::Session(outer_dispatch_param));
                        }
                        {
                            if input_id == (0 + 1 + 1 + 1 + 1) {
                                let outer_dispatch_param =
                                    ::dispatch::Decode::decode(input)?;
                                return Some(Call::Staking(outer_dispatch_param));
                            }
                            {
                                if input_id == (0 + 1 + 1 + 1 + 1 + 1) {
                                    let outer_dispatch_param =
                                        ::dispatch::Decode::decode(input)?;
                                    return Some(Call::Democracy(outer_dispatch_param));
                                }
                                {
                                    if input_id == (0 + 1 + 1 + 1 + 1 + 1 + 1)
                                       {
                                        let outer_dispatch_param =
                                            ::dispatch::Decode::decode(input)?;
                                        return Some(Call::Council(outer_dispatch_param));
                                    }
                                    {
                                        if input_id ==
                                               (0 + 1 + 1 + 1 + 1 + 1 + 1 + 1)
                                           {
                                            let outer_dispatch_param =
                                                ::dispatch::Decode::decode(input)?;
                                            return Some(Call::CouncilVoting(outer_dispatch_param));
                                        }
                                        {
                                            if input_id ==
                                                   (0 + 1 + 1 + 1 + 1 + 1 + 1
                                                        + 1 + 1) {
                                                let outer_dispatch_param =
                                                    ::dispatch::Decode::decode(input)?;
                                                return Some(Call::CouncilMotions(outer_dispatch_param));
                                            }
                                            {
                                                if input_id ==
                                                       (0 + 1 + 1 + 1 + 1 + 1
                                                            + 1 + 1 + 1 + 1) {
                                                    let outer_dispatch_param =
                                                        ::dispatch::Decode::decode(input)?;
                                                    return Some(Call::Treasury(outer_dispatch_param));
                                                }
                                                {
                                                    if input_id ==
                                                           (0 + 1 + 1 + 1 + 1
                                                                + 1 + 1 + 1 +
                                                                1 + 1 + 1) {
                                                        let outer_dispatch_param =
                                                            ::dispatch::Decode::decode(input)?;
                                                        return Some(Call::Contract(outer_dispatch_param));
                                                    }
                                                    {
                                                        if input_id ==
                                                               (0 + 1 + 1 + 1
                                                                    + 1 + 1 +
                                                                    1 + 1 + 1
                                                                    + 1 + 1 +
                                                                    1) {
                                                            let outer_dispatch_param =
                                                                ::dispatch::Decode::decode(input)?;
                                                            return Some(Call::TokenBalances(outer_dispatch_param));
                                                        }
                                                        {
                                                            if input_id ==
                                                                   (0 + 1 + 1
                                                                        + 1 +
                                                                        1 + 1
                                                                        + 1 +
                                                                        1 + 1
                                                                        + 1 +
                                                                        1 + 1
                                                                        + 1) {
                                                                let outer_dispatch_param =
                                                                    ::dispatch::Decode::decode(input)?;
                                                                return Some(Call::FinancialRecords(outer_dispatch_param));
                                                            }
                                                            {
                                                                if input_id ==
                                                                       (0 + 1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1)
                                                                   {
                                                                    let outer_dispatch_param =
                                                                        ::dispatch::Decode::decode(input)?;
                                                                    return Some(Call::MultiSig(outer_dispatch_param));
                                                                }
                                                                None
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
impl ::dispatch::Encode for Call {
    fn encode_to<W: ::dispatch::Output>(&self, dest: &mut W) {
        {
            if let Call::Consensus(ref outer_dispatch_param) = *self {
                dest.push_byte((0) as u8);
                outer_dispatch_param.encode_to(dest);
            }
            {
                if let Call::Balances(ref outer_dispatch_param) = *self {
                    dest.push_byte((0 + 1) as u8);
                    outer_dispatch_param.encode_to(dest);
                }
                {
                    if let Call::Timestamp(ref outer_dispatch_param) = *self {
                        dest.push_byte((0 + 1 + 1) as u8);
                        outer_dispatch_param.encode_to(dest);
                    }
                    {
                        if let Call::Session(ref outer_dispatch_param) = *self
                               {
                            dest.push_byte((0 + 1 + 1 + 1) as u8);
                            outer_dispatch_param.encode_to(dest);
                        }
                        {
                            if let Call::Staking(ref outer_dispatch_param) =
                                   *self {
                                dest.push_byte((0 + 1 + 1 + 1 + 1) as u8);
                                outer_dispatch_param.encode_to(dest);
                            }
                            {
                                if let Call::Democracy(ref outer_dispatch_param)
                                       = *self {
                                    dest.push_byte((0 + 1 + 1 + 1 + 1 + 1) as
                                                       u8);
                                    outer_dispatch_param.encode_to(dest);
                                }
                                {
                                    if let Call::Council(ref outer_dispatch_param)
                                           = *self {
                                        dest.push_byte((0 + 1 + 1 + 1 + 1 + 1
                                                            + 1) as u8);
                                        outer_dispatch_param.encode_to(dest);
                                    }
                                    {
                                        if let Call::CouncilVoting(ref outer_dispatch_param)
                                               = *self {
                                            dest.push_byte((0 + 1 + 1 + 1 + 1
                                                                + 1 + 1 + 1)
                                                               as u8);
                                            outer_dispatch_param.encode_to(dest);
                                        }
                                        {
                                            if let Call::CouncilMotions(ref outer_dispatch_param)
                                                   = *self {
                                                dest.push_byte((0 + 1 + 1 + 1
                                                                    + 1 + 1 +
                                                                    1 + 1 + 1)
                                                                   as u8);
                                                outer_dispatch_param.encode_to(dest);
                                            }
                                            {
                                                if let Call::Treasury(ref outer_dispatch_param)
                                                       = *self {
                                                    dest.push_byte((0 + 1 + 1
                                                                        + 1 +
                                                                        1 + 1
                                                                        + 1 +
                                                                        1 + 1
                                                                        + 1)
                                                                       as u8);
                                                    outer_dispatch_param.encode_to(dest);
                                                }
                                                {
                                                    if let Call::Contract(ref outer_dispatch_param)
                                                           = *self {
                                                        dest.push_byte((0 + 1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1
                                                                            +
                                                                            1)
                                                                           as
                                                                           u8);
                                                        outer_dispatch_param.encode_to(dest);
                                                    }
                                                    {
                                                        if let Call::TokenBalances(ref outer_dispatch_param)
                                                               = *self {
                                                            dest.push_byte((0
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1
                                                                                +
                                                                                1)
                                                                               as
                                                                               u8);
                                                            outer_dispatch_param.encode_to(dest);
                                                        }
                                                        {
                                                            if let Call::FinancialRecords(ref outer_dispatch_param)
                                                                   = *self {
                                                                dest.push_byte((0
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1
                                                                                    +
                                                                                    1)
                                                                                   as
                                                                                   u8);
                                                                outer_dispatch_param.encode_to(dest);
                                                            }
                                                            {
                                                                if let Call::MultiSig(ref outer_dispatch_param)
                                                                       = *self
                                                                       {
                                                                    dest.push_byte((0
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1
                                                                                        +
                                                                                        1)
                                                                                       as
                                                                                       u8);
                                                                    outer_dispatch_param.encode_to(dest);
                                                                }
                                                                { }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
impl ::dispatch::Dispatchable for Call {
    type
    Origin
    =
    Origin;
    type
    Trait
    =
    Call;
    fn dispatch(self, origin: Origin) -> ::dispatch::Result {
        match self {
            Call::Consensus(call) => call.dispatch(origin),
            Call::Balances(call) => call.dispatch(origin),
            Call::Timestamp(call) => call.dispatch(origin),
            Call::Session(call) => call.dispatch(origin),
            Call::Staking(call) => call.dispatch(origin),
            Call::Democracy(call) => call.dispatch(origin),
            Call::Council(call) => call.dispatch(origin),
            Call::CouncilVoting(call) => call.dispatch(origin),
            Call::CouncilMotions(call) => call.dispatch(origin),
            Call::Treasury(call) => call.dispatch(origin),
            Call::Contract(call) => call.dispatch(origin),
            Call::TokenBalances(call) => call.dispatch(origin),
            Call::FinancialRecords(call) => call.dispatch(origin),
            Call::MultiSig(call) => call.dispatch(origin),
        }
    }
}
impl ::dispatch::IsSubType<Consensus> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Consensus as ::dispatch::Callable>::Call> {
        if let Call::Consensus(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Balances> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Balances as ::dispatch::Callable>::Call> {
        if let Call::Balances(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Timestamp> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Timestamp as ::dispatch::Callable>::Call> {
        if let Call::Timestamp(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Session> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Session as ::dispatch::Callable>::Call> {
        if let Call::Session(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Staking> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Staking as ::dispatch::Callable>::Call> {
        if let Call::Staking(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Democracy> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Democracy as ::dispatch::Callable>::Call> {
        if let Call::Democracy(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Council> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Council as ::dispatch::Callable>::Call> {
        if let Call::Council(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<CouncilVoting> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<CouncilVoting as ::dispatch::Callable>::Call> {
        if let Call::CouncilVoting(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<CouncilMotions> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<CouncilMotions as ::dispatch::Callable>::Call> {
        if let Call::CouncilMotions(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Treasury> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Treasury as ::dispatch::Callable>::Call> {
        if let Call::Treasury(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<Contract> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<Contract as ::dispatch::Callable>::Call> {
        if let Call::Contract(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<TokenBalances> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<TokenBalances as ::dispatch::Callable>::Call> {
        if let Call::TokenBalances(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<FinancialRecords> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<FinancialRecords as ::dispatch::Callable>::Call> {
        if let Call::FinancialRecords(ref r) = *self { Some(r) } else { None }
    }
}
impl ::dispatch::IsSubType<MultiSig> for Call {
    fn is_aux_sub_type(&self)
     -> Option<&<MultiSig as ::dispatch::Callable>::Call> {
        if let Call::MultiSig(ref r) = *self { Some(r) } else { None }
    }
}
impl Runtime {
    pub fn metadata() -> ::metadata::RuntimeMetadata {
        ::metadata::RuntimeMetadata{outer_event: Self::outer_event_metadata(),
                                    modules:
                                        ::metadata::DecodeDifferent::Encode(&[::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("system"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(system::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(system::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("consensus"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(consensus::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(consensus::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("balances"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(balances::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(balances::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("timestamp"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(timestamp::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(timestamp::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("session"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(session::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(session::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("staking"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(staking::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(staking::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("democracy"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(democracy::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(democracy::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("council"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(council::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(council::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("council_voting"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(council_voting::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(council_voting::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("council_motions"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(council_motions::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(council_motions::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("treasury"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(treasury::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(treasury::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("contract"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(contract::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    None,},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("tokenbalances"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(tokenbalances::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(tokenbalances::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("financialrecords"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(financialrecords::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(financialrecords::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("multisig"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(multisig::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    Some(::metadata::DecodeDifferent::Encode(::metadata::FnEncode(multisig::Module::<Runtime>::store_metadata))),},
                                                                              ::metadata::RuntimeModuleMetadata{prefix:
                                                                                                                    ::metadata::DecodeDifferent::Encode("cxsupport"),
                                                                                                                module:
                                                                                                                    ::metadata::DecodeDifferent::Encode(::metadata::FnEncode(cxsupport::Module::<Runtime>::metadata)),
                                                                                                                storage:
                                                                                                                    None,}]),}
    }
}
/// Wrapper for all possible log entries for the `$trait` runtime. Provides binary-compatible
/// `Encode`/`Decode` implementations with the corresponding `generic::DigestItem`.
#[allow(non_camel_case_types)]
#[structural_match]
pub struct Log(InternalLog);
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::clone::Clone for Log {
    #[inline]
    fn clone(&self) -> Log {
        match *self {
            Log(ref __self_0_0) =>
            Log(::std::clone::Clone::clone(&(*__self_0_0))),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::PartialEq for Log {
    #[inline]
    fn eq(&self, other: &Log) -> bool {
        match *other {
            Log(ref __self_1_0) =>
            match *self {
                Log(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Log) -> bool {
        match *other {
            Log(ref __self_1_0) =>
            match *self {
                Log(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::Eq for Log {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        { let _: ::std::cmp::AssertParamIsEq<InternalLog>; }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::fmt::Debug for Log {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Log(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Log");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_Log: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl _serde::Serialize for Log {
            fn serialize<__S>(&self, __serializer: __S)
             -> _serde::export::Result<__S::Ok, __S::Error> where
             __S: _serde::Serializer {
                _serde::Serializer::serialize_newtype_struct(__serializer,
                                                             "Log", &self.0)
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_Log: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl <'de> _serde::Deserialize<'de> for Log {
            fn deserialize<__D>(__deserializer: __D)
             -> _serde::export::Result<Self, __D::Error> where
             __D: _serde::Deserializer<'de> {
                struct __Visitor<'de> {
                    marker: _serde::export::PhantomData<Log>,
                    lifetime: _serde::export::PhantomData<&'de ()>,
                }
                impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type
                    Value
                    =
                    Log;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "tuple struct Log")
                    }
                    #[inline]
                    fn visit_newtype_struct<__E>(self, __e: __E)
                     -> _serde::export::Result<Self::Value, __E::Error> where
                     __E: _serde::Deserializer<'de> {
                        let __field0: InternalLog =
                            match <InternalLog as
                                      _serde::Deserialize>::deserialize(__e) {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            };
                        _serde::export::Ok(Log(__field0))
                    }
                    #[inline]
                    fn visit_seq<__A>(self, mut __seq: __A)
                     -> _serde::export::Result<Self::Value, __A::Error> where
                     __A: _serde::de::SeqAccess<'de> {
                        let __field0 =
                            match match _serde::de::SeqAccess::next_element::<InternalLog>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                 &"tuple struct Log with 1 element"));
                                }
                            };
                        _serde::export::Ok(Log(__field0))
                    }
                }
                _serde::Deserializer::deserialize_newtype_struct(__deserializer,
                                                                 "Log",
                                                                 __Visitor{marker:
                                                                               _serde::export::PhantomData::<Log>,
                                                                           lifetime:
                                                                               _serde::export::PhantomData,})
            }
        }
    };
/// All possible log entries for the `$trait` runtime. `Encode`/`Decode` implementations
/// are auto-generated => it is not binary-compatible with `generic::DigestItem`.
#[allow(non_camel_case_types)]
#[structural_match]
enum InternalLog {
    system(system::Log<Runtime>),
    consensus(consensus::Log<Runtime>),
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::clone::Clone for InternalLog {
    #[inline]
    fn clone(&self) -> InternalLog {
        match (&*self,) {
            (&InternalLog::system(ref __self_0),) =>
            InternalLog::system(::std::clone::Clone::clone(&(*__self_0))),
            (&InternalLog::consensus(ref __self_0),) =>
            InternalLog::consensus(::std::clone::Clone::clone(&(*__self_0))),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::PartialEq for InternalLog {
    #[inline]
    fn eq(&self, other: &InternalLog) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&InternalLog::system(ref __self_0),
                     &InternalLog::system(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    (&InternalLog::consensus(ref __self_0),
                     &InternalLog::consensus(ref __arg_1_0)) =>
                    (*__self_0) == (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { false }
        }
    }
    #[inline]
    fn ne(&self, other: &InternalLog) -> bool {
        {
            let __self_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*self) } as
                    isize;
            let __arg_1_vi =
                unsafe { ::std::intrinsics::discriminant_value(&*other) } as
                    isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&InternalLog::system(ref __self_0),
                     &InternalLog::system(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    (&InternalLog::consensus(ref __self_0),
                     &InternalLog::consensus(ref __arg_1_0)) =>
                    (*__self_0) != (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() }
                }
            } else { true }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::cmp::Eq for InternalLog {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::std::cmp::AssertParamIsEq<system::Log<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<consensus::Log<Runtime>>;
        }
    }
}
impl ::codec::Encode for InternalLog {
    fn encode_to<EncOut: ::codec::Output>(&self, dest: &mut EncOut) {
        match *self {
            InternalLog::system(ref aa) => {
                dest.push_byte(0usize as u8);
                dest.push(aa);
            }
            InternalLog::consensus(ref aa) => {
                dest.push_byte(1usize as u8);
                dest.push(aa);
            }
        }
    }
}
impl ::codec::Decode for InternalLog {
    fn decode<DecIn: ::codec::Input>(input: &mut DecIn) -> Option<Self> {
        match input.read_byte()? {
            x if x == 0usize as u8 => {
                Some(InternalLog::system(::codec::Decode::decode(input)?))
            }
            x if x == 1usize as u8 => {
                Some(InternalLog::consensus(::codec::Decode::decode(input)?))
            }
            _ => None,
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::fmt::Debug for InternalLog {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match (&*self,) {
            (&InternalLog::system(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("system");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&InternalLog::consensus(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("consensus");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_InternalLog: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl _serde::Serialize for InternalLog {
            fn serialize<__S>(&self, __serializer: __S)
             -> _serde::export::Result<__S::Ok, __S::Error> where
             __S: _serde::Serializer {
                match *self {
                    InternalLog::system(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "InternalLog",
                                                                  0u32,
                                                                  "system",
                                                                  __field0),
                    InternalLog::consensus(ref __field0) =>
                    _serde::Serializer::serialize_newtype_variant(__serializer,
                                                                  "InternalLog",
                                                                  1u32,
                                                                  "consensus",
                                                                  __field0),
                }
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_InternalLog: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl <'de> _serde::Deserialize<'de> for InternalLog {
            fn deserialize<__D>(__deserializer: __D)
             -> _serde::export::Result<Self, __D::Error> where
             __D: _serde::Deserializer<'de> {
                #[allow(non_camel_case_types)]
                enum __Field { __field0, __field1, }
                struct __FieldVisitor;
                impl <'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type
                    Value
                    =
                    __Field;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "variant identifier")
                    }
                    fn visit_u64<__E>(self, __value: u64)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            0u64 => _serde::export::Ok(__Field::__field0),
                            1u64 => _serde::export::Ok(__Field::__field1),
                            _ =>
                            _serde::export::Err(_serde::de::Error::invalid_value(_serde::de::Unexpected::Unsigned(__value),
                                                                                 &"variant index 0 <= i < 2")),
                        }
                    }
                    fn visit_str<__E>(self, __value: &str)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            "system" => _serde::export::Ok(__Field::__field0),
                            "consensus" =>
                            _serde::export::Ok(__Field::__field1),
                            _ => {
                                _serde::export::Err(_serde::de::Error::unknown_variant(__value,
                                                                                       VARIANTS))
                            }
                        }
                    }
                    fn visit_bytes<__E>(self, __value: &[u8])
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            b"system" =>
                            _serde::export::Ok(__Field::__field0),
                            b"consensus" =>
                            _serde::export::Ok(__Field::__field1),
                            _ => {
                                let __value =
                                    &_serde::export::from_utf8_lossy(__value);
                                _serde::export::Err(_serde::de::Error::unknown_variant(__value,
                                                                                       VARIANTS))
                            }
                        }
                    }
                }
                impl <'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::export::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                     __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::export::PhantomData<InternalLog>,
                    lifetime: _serde::export::PhantomData<&'de ()>,
                }
                impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type
                    Value
                    =
                    InternalLog;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "enum InternalLog")
                    }
                    fn visit_enum<__A>(self, __data: __A)
                     -> _serde::export::Result<Self::Value, __A::Error> where
                     __A: _serde::de::EnumAccess<'de> {
                        match match _serde::de::EnumAccess::variant(__data) {
                                  _serde::export::Ok(__val) => __val,
                                  _serde::export::Err(__err) => {
                                      return _serde::export::Err(__err);
                                  }
                              } {
                            (__Field::__field0, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<system::Log<Runtime>>(__variant),
                                                        InternalLog::system),
                            (__Field::__field1, __variant) =>
                            _serde::export::Result::map(_serde::de::VariantAccess::newtype_variant::<consensus::Log<Runtime>>(__variant),
                                                        InternalLog::consensus),
                        }
                    }
                }
                const VARIANTS: &'static [&'static str] =
                    &["system", "consensus"];
                _serde::Deserializer::deserialize_enum(__deserializer,
                                                       "InternalLog",
                                                       VARIANTS,
                                                       __Visitor{marker:
                                                                     _serde::export::PhantomData::<InternalLog>,
                                                                 lifetime:
                                                                     _serde::export::PhantomData,})
            }
        }
    };
impl Log {
    /// Try to convert `$name` into `generic::DigestItemRef`. Returns Some when
    /// `self` is a 'system' log && it has been marked as 'system' in macro call.
    /// Otherwise, None is returned.
    #[allow(unreachable_patterns)]
    fn dref<'a>(&'a self)
     -> Option<::generic::DigestItemRef<'a, Hash, SessionKey>> {
        match self.0 {
            InternalLog::system(system::RawLog::ChangesTrieRoot(ref v)) =>
            Some(::generic::DigestItemRef::ChangesTrieRoot(v)),
            InternalLog::consensus(consensus::RawLog::AuthoritiesChange(ref v))
            => Some(::generic::DigestItemRef::AuthoritiesChange(v)),
            _ => None,
        }
    }
}
impl From<::generic::DigestItem<Hash, SessionKey>> for Log {
    /// Converts `generic::DigestItem` into `$name`. If `generic::DigestItem` represents
    /// a system item which is supported by the runtime, it is returned.
    /// Otherwise we expect a `Other` log item. Trying to convert from anything other
    /// will lead to panic in runtime, since the runtime does not supports this 'system'
    /// log item.
    #[allow(unreachable_patterns)]
    fn from(gen: ::generic::DigestItem<Hash, SessionKey>) -> Self {
        match gen {
            ::generic::DigestItem::ChangesTrieRoot(value) =>
            Log(InternalLog::system(system::RawLog::ChangesTrieRoot(value))),
            ::generic::DigestItem::AuthoritiesChange(value) =>
            Log(InternalLog::consensus(consensus::RawLog::AuthoritiesChange(value))),
            _ =>
            gen.as_other().and_then(|value|
                                        ::codec::Decode::decode(&mut &value[..])).map(Log).expect("not allowed to fail in runtime"),
        }
    }
}
impl ::codec::Decode for Log {
    /// `generic::DigestItem` binray compatible decode.
    fn decode<I: ::codec::Input>(input: &mut I) -> Option<Self> {
        let gen: ::generic::DigestItem<Hash, SessionKey> =
            ::codec::Decode::decode(input)?;
        Some(Log::from(gen))
    }
}
impl ::codec::Encode for Log {
    /// `generic::DigestItem` binray compatible encode.
    fn encode(&self) -> Vec<u8> {
        match self.dref() {
            Some(dref) => dref.encode(),
            None => {
                let gen: ::generic::DigestItem<Hash, SessionKey> =
                    ::generic::DigestItem::Other(self.0.encode());
                gen.encode()
            }
        }
    }
}
impl From<system::Log<Runtime>> for Log {
    /// Converts single module log item into `$name`.
    fn from(x: system::Log<Runtime>) -> Self { Log(x.into()) }
}
impl From<system::Log<Runtime>> for InternalLog {
    /// Converts single module log item into `$internal`.
    fn from(x: system::Log<Runtime>) -> Self { InternalLog::system(x) }
}
impl From<consensus::Log<Runtime>> for Log {
    /// Converts single module log item into `$name`.
    fn from(x: consensus::Log<Runtime>) -> Self { Log(x.into()) }
}
impl From<consensus::Log<Runtime>> for InternalLog {
    /// Converts single module log item into `$internal`.
    fn from(x: consensus::Log<Runtime>) -> Self { InternalLog::consensus(x) }
}
#[allow(unused)]
enum ProcMacroHack {
    Input =
        ("substrate_generate_config_name [ \"config-name\" System ] = System Config ;\nsubstrate_generate_config_name [ \"config-name\" Consensus ] = Consensus Config\n; substrate_generate_config_name [ \"config-name\" Balances ] = Balances Config\n; substrate_generate_config_name [ \"config-name\" Timestamp ] = Timestamp\nConfig ; substrate_generate_config_name [ \"config-name\" Session ] = Session\nConfig ; substrate_generate_config_name [ \"config-name\" Staking ] = Staking\nConfig ; substrate_generate_config_name [ \"config-name\" Democracy ] =\nDemocracy Config ; substrate_generate_config_name [ \"config-name\" Council ] =\nCouncil Config ; substrate_generate_config_name [ \"config-name\" Treasury ] =\nTreasury Config ; substrate_generate_config_name [ \"config-name\" Contract ] =\nContract Config ; substrate_generate_config_name [ \"config-name\" TokenBalances\n] = TokenBalances Config ; substrate_generate_config_name [\n\"config-name\" FinancialRecords ] = FinancialRecords Config ;\nsubstrate_generate_config_name [ \"config-name\" MultiSig ] = MultiSig Config ;",
         0).1,
}
macro_rules! substrate_generate_config_name((
                                            @ ( $ ( $ v : tt ) * ) (
                                            $ ( $ stack : tt ) * ) (
                                            $ ( $ first : tt ) * ) $ (
                                            $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ ( $ ( $ v ) * ) (
                                            (  ) $ ( $ stack ) * ) $ ( $ first
                                            ) * __mashup_close_paren $ (
                                            $ rest ) * } } ; (
                                            @ ( $ ( $ v : tt ) * ) (
                                            $ ( $ stack : tt ) * ) [
                                            $ ( $ first : tt ) * ] $ (
                                            $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ ( $ ( $ v ) * ) (
                                            (  ) $ ( $ stack ) * ) $ ( $ first
                                            ) * __mashup_close_bracket $ (
                                            $ rest ) * } } ; (
                                            @ ( $ ( $ v : tt ) * ) (
                                            $ ( $ stack : tt ) * ) {
                                            $ ( $ first : tt ) * } $ (
                                            $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ ( $ ( $ v ) * ) (
                                            (  ) $ ( $ stack ) * ) $ ( $ first
                                            ) * __mashup_close_brace $ (
                                            $ rest ) * } } ; (
                                            @ ( $ ( $ v : tt ) * ) (
                                            ( $ ( $ close : tt ) * ) (
                                            $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * )
                                            __mashup_close_paren $ (
                                            $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ ( $ ( $ v ) * ) (
                                            (
                                            $ ( $ top ) * ( $ ( $ close ) * )
                                            ) $ ( $ stack ) * ) $ ( $ rest ) *
                                            } } ; (
                                            @ ( $ ( $ v : tt ) * ) (
                                            ( $ ( $ close : tt ) * ) (
                                            $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * )
                                            __mashup_close_bracket $ (
                                            $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ ( $ ( $ v ) * ) (
                                            (
                                            $ ( $ top ) * [ $ ( $ close ) * ]
                                            ) $ ( $ stack ) * ) $ ( $ rest ) *
                                            } } ; (
                                            @ ( $ ( $ v : tt ) * ) (
                                            ( $ ( $ close : tt ) * ) (
                                            $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * )
                                            __mashup_close_brace $ (
                                            $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ ( $ ( $ v ) * ) (
                                            (
                                            $ ( $ top ) * { $ ( $ close ) * }
                                            ) $ ( $ stack ) * ) $ ( $ rest ) *
                                            } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            System $ ( $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v0 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Consensus $ ( $ rest : tt ) * ) =>
                                            {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v1 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Balances $ ( $ rest : tt ) * ) =>
                                            {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v2 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Timestamp $ ( $ rest : tt ) * ) =>
                                            {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v3 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Session $ ( $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v4 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Staking $ ( $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v5 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Democracy $ ( $ rest : tt ) * ) =>
                                            {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v6 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Council $ ( $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v7 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Treasury $ ( $ rest : tt ) * ) =>
                                            {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v8 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            Contract $ ( $ rest : tt ) * ) =>
                                            {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v9 ) $ ( $ stack
                                            ) * ) $ ( $ rest ) * } } ; (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            TokenBalances $ ( $ rest : tt ) *
                                            ) => {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v10 ) $ (
                                            $ stack ) * ) $ ( $ rest ) * } } ;
                                            (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            FinancialRecords $ ( $ rest : tt )
                                            * ) => {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v11 ) $ (
                                            $ stack ) * ) $ ( $ rest ) * } } ;
                                            (
                                            @ (
                                            $ v0 : tt $ v1 : tt $ v2 : tt $ v3
                                            : tt $ v4 : tt $ v5 : tt $ v6 : tt
                                            $ v7 : tt $ v8 : tt $ v9 : tt $
                                            v10 : tt $ v11 : tt $ v12 : tt ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) "config-name"
                                            MultiSig $ ( $ rest : tt ) * ) =>
                                            {
                                            substrate_generate_config_name ! {
                                            @ (
                                            $ v0 $ v1 $ v2 $ v3 $ v4 $ v5 $ v6
                                            $ v7 $ v8 $ v9 $ v10 $ v11 $ v12 )
                                            (
                                            ( $ ( $ top ) * $ v12 ) $ (
                                            $ stack ) * ) $ ( $ rest ) * } } ;
                                            (
                                            @ ( $ ( $ v : tt ) * ) (
                                            ( $ ( $ top : tt ) * ) $ (
                                            $ stack : tt ) * ) $ first : tt $
                                            ( $ rest : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ ( $ ( $ v ) * ) (
                                            ( $ ( $ top ) * $ first ) $ (
                                            $ stack ) * ) $ ( $ rest ) * } } ;
                                            (
                                            @ ( $ ( $ v : tt ) * ) (
                                            ( $ ( $ top : tt ) + ) ) ) => {
                                            $ ( $ top ) + } ; (
                                            $ ( $ tt : tt ) * ) => {
                                            substrate_generate_config_name ! {
                                            @ (
                                            SystemConfig ConsensusConfig
                                            BalancesConfig TimestampConfig
                                            SessionConfig StakingConfig
                                            DemocracyConfig CouncilConfig
                                            TreasuryConfig ContractConfig
                                            TokenBalancesConfig
                                            FinancialRecordsConfig
                                            MultiSigConfig ) ( (  ) ) $ ( $ tt
                                            ) * } });
#[cfg(any(feature = "std", test))]
pub type SystemConfig = system::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type ConsensusConfig = consensus::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type BalancesConfig = balances::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type TimestampConfig = timestamp::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type SessionConfig = session::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type StakingConfig = staking::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type DemocracyConfig = democracy::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type CouncilConfig = council::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type TreasuryConfig = treasury::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type ContractConfig = contract::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type TokenBalancesConfig = tokenbalances::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type FinancialRecordsConfig = financialrecords::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type MultiSigConfig = multisig::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct GenesisConfig {
    pub system: Option<SystemConfig>,
    pub consensus: Option<ConsensusConfig>,
    pub balances: Option<BalancesConfig>,
    pub timestamp: Option<TimestampConfig>,
    pub session: Option<SessionConfig>,
    pub staking: Option<StakingConfig>,
    pub democracy: Option<DemocracyConfig>,
    pub council: Option<CouncilConfig>,
    pub treasury: Option<TreasuryConfig>,
    pub contract: Option<ContractConfig>,
    pub tokenbalances: Option<TokenBalancesConfig>,
    pub financialrecords: Option<FinancialRecordsConfig>,
    pub multisig: Option<MultiSigConfig>,
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_GenesisConfig: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl _serde::Serialize for GenesisConfig {
            fn serialize<__S>(&self, __serializer: __S)
             -> _serde::export::Result<__S::Ok, __S::Error> where
             __S: _serde::Serializer {
                let mut __serde_state =
                    match _serde::Serializer::serialize_struct(__serializer,
                                                               "GenesisConfig",
                                                               0 + 1 + 1 + 1 +
                                                                   1 + 1 + 1 +
                                                                   1 + 1 + 1 +
                                                                   1 + 1 + 1 +
                                                                   1) {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "system",
                                                                    &self.system)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "consensus",
                                                                    &self.consensus)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "balances",
                                                                    &self.balances)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "timestamp",
                                                                    &self.timestamp)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "session",
                                                                    &self.session)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "staking",
                                                                    &self.staking)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "democracy",
                                                                    &self.democracy)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "council",
                                                                    &self.council)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "treasury",
                                                                    &self.treasury)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "contract",
                                                                    &self.contract)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "tokenbalances",
                                                                    &self.tokenbalances)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "financialrecords",
                                                                    &self.financialrecords)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(&mut __serde_state,
                                                                    "multisig",
                                                                    &self.multisig)
                    {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_GenesisConfig: () =
    {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate serde as _serde;
        #[allow(unused_macros)]
        macro_rules! try(( $ __expr : expr ) => {
                         match $ __expr {
                         _serde :: export :: Ok ( __val ) => __val , _serde ::
                         export :: Err ( __err ) => {
                         return _serde :: export :: Err ( __err ) ; } } });
        #[automatically_derived]
        impl <'de> _serde::Deserialize<'de> for GenesisConfig {
            fn deserialize<__D>(__deserializer: __D)
             -> _serde::export::Result<Self, __D::Error> where
             __D: _serde::Deserializer<'de> {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __field3,
                    __field4,
                    __field5,
                    __field6,
                    __field7,
                    __field8,
                    __field9,
                    __field10,
                    __field11,
                    __field12,
                }
                struct __FieldVisitor;
                impl <'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type
                    Value
                    =
                    __Field;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "field identifier")
                    }
                    fn visit_u64<__E>(self, __value: u64)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            0u64 => _serde::export::Ok(__Field::__field0),
                            1u64 => _serde::export::Ok(__Field::__field1),
                            2u64 => _serde::export::Ok(__Field::__field2),
                            3u64 => _serde::export::Ok(__Field::__field3),
                            4u64 => _serde::export::Ok(__Field::__field4),
                            5u64 => _serde::export::Ok(__Field::__field5),
                            6u64 => _serde::export::Ok(__Field::__field6),
                            7u64 => _serde::export::Ok(__Field::__field7),
                            8u64 => _serde::export::Ok(__Field::__field8),
                            9u64 => _serde::export::Ok(__Field::__field9),
                            10u64 => _serde::export::Ok(__Field::__field10),
                            11u64 => _serde::export::Ok(__Field::__field11),
                            12u64 => _serde::export::Ok(__Field::__field12),
                            _ =>
                            _serde::export::Err(_serde::de::Error::invalid_value(_serde::de::Unexpected::Unsigned(__value),
                                                                                 &"field index 0 <= i < 13")),
                        }
                    }
                    fn visit_str<__E>(self, __value: &str)
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            "system" => _serde::export::Ok(__Field::__field0),
                            "consensus" =>
                            _serde::export::Ok(__Field::__field1),
                            "balances" =>
                            _serde::export::Ok(__Field::__field2),
                            "timestamp" =>
                            _serde::export::Ok(__Field::__field3),
                            "session" =>
                            _serde::export::Ok(__Field::__field4),
                            "staking" =>
                            _serde::export::Ok(__Field::__field5),
                            "democracy" =>
                            _serde::export::Ok(__Field::__field6),
                            "council" =>
                            _serde::export::Ok(__Field::__field7),
                            "treasury" =>
                            _serde::export::Ok(__Field::__field8),
                            "contract" =>
                            _serde::export::Ok(__Field::__field9),
                            "tokenbalances" =>
                            _serde::export::Ok(__Field::__field10),
                            "financialrecords" =>
                            _serde::export::Ok(__Field::__field11),
                            "multisig" =>
                            _serde::export::Ok(__Field::__field12),
                            _ => {
                                _serde::export::Err(_serde::de::Error::unknown_field(__value,
                                                                                     FIELDS))
                            }
                        }
                    }
                    fn visit_bytes<__E>(self, __value: &[u8])
                     -> _serde::export::Result<Self::Value, __E> where
                     __E: _serde::de::Error {
                        match __value {
                            b"system" =>
                            _serde::export::Ok(__Field::__field0),
                            b"consensus" =>
                            _serde::export::Ok(__Field::__field1),
                            b"balances" =>
                            _serde::export::Ok(__Field::__field2),
                            b"timestamp" =>
                            _serde::export::Ok(__Field::__field3),
                            b"session" =>
                            _serde::export::Ok(__Field::__field4),
                            b"staking" =>
                            _serde::export::Ok(__Field::__field5),
                            b"democracy" =>
                            _serde::export::Ok(__Field::__field6),
                            b"council" =>
                            _serde::export::Ok(__Field::__field7),
                            b"treasury" =>
                            _serde::export::Ok(__Field::__field8),
                            b"contract" =>
                            _serde::export::Ok(__Field::__field9),
                            b"tokenbalances" =>
                            _serde::export::Ok(__Field::__field10),
                            b"financialrecords" =>
                            _serde::export::Ok(__Field::__field11),
                            b"multisig" =>
                            _serde::export::Ok(__Field::__field12),
                            _ => {
                                let __value =
                                    &_serde::export::from_utf8_lossy(__value);
                                _serde::export::Err(_serde::de::Error::unknown_field(__value,
                                                                                     FIELDS))
                            }
                        }
                    }
                }
                impl <'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::export::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                     __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::export::PhantomData<GenesisConfig>,
                    lifetime: _serde::export::PhantomData<&'de ()>,
                }
                impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type
                    Value
                    =
                    GenesisConfig;
                    fn expecting(&self,
                                 __formatter: &mut _serde::export::Formatter)
                     -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter,
                                                             "struct GenesisConfig")
                    }
                    #[inline]
                    fn visit_seq<__A>(self, mut __seq: __A)
                     -> _serde::export::Result<Self::Value, __A::Error> where
                     __A: _serde::de::SeqAccess<'de> {
                        let __field0 =
                            match match _serde::de::SeqAccess::next_element::<Option<SystemConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field1 =
                            match match _serde::de::SeqAccess::next_element::<Option<ConsensusConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field2 =
                            match match _serde::de::SeqAccess::next_element::<Option<BalancesConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(2usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field3 =
                            match match _serde::de::SeqAccess::next_element::<Option<TimestampConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(3usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field4 =
                            match match _serde::de::SeqAccess::next_element::<Option<SessionConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(4usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field5 =
                            match match _serde::de::SeqAccess::next_element::<Option<StakingConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(5usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field6 =
                            match match _serde::de::SeqAccess::next_element::<Option<DemocracyConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(6usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field7 =
                            match match _serde::de::SeqAccess::next_element::<Option<CouncilConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(7usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field8 =
                            match match _serde::de::SeqAccess::next_element::<Option<TreasuryConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(8usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field9 =
                            match match _serde::de::SeqAccess::next_element::<Option<ContractConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(9usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field10 =
                            match match _serde::de::SeqAccess::next_element::<Option<TokenBalancesConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(10usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field11 =
                            match match _serde::de::SeqAccess::next_element::<Option<FinancialRecordsConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(11usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        let __field12 =
                            match match _serde::de::SeqAccess::next_element::<Option<MultiSigConfig>>(&mut __seq)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(12usize,
                                                                                                 &"struct GenesisConfig with 13 elements"));
                                }
                            };
                        _serde::export::Ok(GenesisConfig{system: __field0,
                                                         consensus: __field1,
                                                         balances: __field2,
                                                         timestamp: __field3,
                                                         session: __field4,
                                                         staking: __field5,
                                                         democracy: __field6,
                                                         council: __field7,
                                                         treasury: __field8,
                                                         contract: __field9,
                                                         tokenbalances:
                                                             __field10,
                                                         financialrecords:
                                                             __field11,
                                                         multisig:
                                                             __field12,})
                    }
                    #[inline]
                    fn visit_map<__A>(self, mut __map: __A)
                     -> _serde::export::Result<Self::Value, __A::Error> where
                     __A: _serde::de::MapAccess<'de> {
                        let mut __field0:
                                _serde::export::Option<Option<SystemConfig>> =
                            _serde::export::None;
                        let mut __field1:
                                _serde::export::Option<Option<ConsensusConfig>> =
                            _serde::export::None;
                        let mut __field2:
                                _serde::export::Option<Option<BalancesConfig>> =
                            _serde::export::None;
                        let mut __field3:
                                _serde::export::Option<Option<TimestampConfig>> =
                            _serde::export::None;
                        let mut __field4:
                                _serde::export::Option<Option<SessionConfig>> =
                            _serde::export::None;
                        let mut __field5:
                                _serde::export::Option<Option<StakingConfig>> =
                            _serde::export::None;
                        let mut __field6:
                                _serde::export::Option<Option<DemocracyConfig>> =
                            _serde::export::None;
                        let mut __field7:
                                _serde::export::Option<Option<CouncilConfig>> =
                            _serde::export::None;
                        let mut __field8:
                                _serde::export::Option<Option<TreasuryConfig>> =
                            _serde::export::None;
                        let mut __field9:
                                _serde::export::Option<Option<ContractConfig>> =
                            _serde::export::None;
                        let mut __field10:
                                _serde::export::Option<Option<TokenBalancesConfig>> =
                            _serde::export::None;
                        let mut __field11:
                                _serde::export::Option<Option<FinancialRecordsConfig>> =
                            _serde::export::None;
                        let mut __field12:
                                _serde::export::Option<Option<MultiSigConfig>> =
                            _serde::export::None;
                        while let _serde::export::Some(__key) =
                                  match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                      {
                                      _serde::export::Ok(__val) => __val,
                                      _serde::export::Err(__err) => {
                                          return _serde::export::Err(__err);
                                      }
                                  } {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::export::Option::is_some(&__field0)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("system"));
                                    }
                                    __field0 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<SystemConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field1 => {
                                    if _serde::export::Option::is_some(&__field1)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("consensus"));
                                    }
                                    __field1 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<ConsensusConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field2 => {
                                    if _serde::export::Option::is_some(&__field2)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("balances"));
                                    }
                                    __field2 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<BalancesConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field3 => {
                                    if _serde::export::Option::is_some(&__field3)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("timestamp"));
                                    }
                                    __field3 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<TimestampConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field4 => {
                                    if _serde::export::Option::is_some(&__field4)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("session"));
                                    }
                                    __field4 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<SessionConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field5 => {
                                    if _serde::export::Option::is_some(&__field5)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("staking"));
                                    }
                                    __field5 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<StakingConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field6 => {
                                    if _serde::export::Option::is_some(&__field6)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("democracy"));
                                    }
                                    __field6 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<DemocracyConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field7 => {
                                    if _serde::export::Option::is_some(&__field7)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("council"));
                                    }
                                    __field7 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<CouncilConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field8 => {
                                    if _serde::export::Option::is_some(&__field8)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("treasury"));
                                    }
                                    __field8 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<TreasuryConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field9 => {
                                    if _serde::export::Option::is_some(&__field9)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("contract"));
                                    }
                                    __field9 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<ContractConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field10 => {
                                    if _serde::export::Option::is_some(&__field10)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("tokenbalances"));
                                    }
                                    __field10 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<TokenBalancesConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field11 => {
                                    if _serde::export::Option::is_some(&__field11)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("financialrecords"));
                                    }
                                    __field11 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<FinancialRecordsConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                                __Field::__field12 => {
                                    if _serde::export::Option::is_some(&__field12)
                                       {
                                        return _serde::export::Err(<__A::Error
                                                                       as
                                                                       _serde::de::Error>::duplicate_field("multisig"));
                                    }
                                    __field12 =
                                        _serde::export::Some(match _serde::de::MapAccess::next_value::<Option<MultiSigConfig>>(&mut __map)
                                                                 {
                                                                 _serde::export::Ok(__val)
                                                                 => __val,
                                                                 _serde::export::Err(__err)
                                                                 => {
                                                                     return _serde::export::Err(__err);
                                                                 }
                                                             });
                                }
                            }
                        }
                        let __field0 =
                            match __field0 {
                                _serde::export::Some(__field0) => __field0,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("system")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field1 =
                            match __field1 {
                                _serde::export::Some(__field1) => __field1,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("consensus")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field2 =
                            match __field2 {
                                _serde::export::Some(__field2) => __field2,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("balances")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field3 =
                            match __field3 {
                                _serde::export::Some(__field3) => __field3,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("timestamp")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field4 =
                            match __field4 {
                                _serde::export::Some(__field4) => __field4,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("session")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field5 =
                            match __field5 {
                                _serde::export::Some(__field5) => __field5,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("staking")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field6 =
                            match __field6 {
                                _serde::export::Some(__field6) => __field6,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("democracy")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field7 =
                            match __field7 {
                                _serde::export::Some(__field7) => __field7,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("council")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field8 =
                            match __field8 {
                                _serde::export::Some(__field8) => __field8,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("treasury")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field9 =
                            match __field9 {
                                _serde::export::Some(__field9) => __field9,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("contract")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field10 =
                            match __field10 {
                                _serde::export::Some(__field10) => __field10,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("tokenbalances")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field11 =
                            match __field11 {
                                _serde::export::Some(__field11) => __field11,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("financialrecords")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        let __field12 =
                            match __field12 {
                                _serde::export::Some(__field12) => __field12,
                                _serde::export::None =>
                                match _serde::private::de::missing_field("multisig")
                                    {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                },
                            };
                        _serde::export::Ok(GenesisConfig{system: __field0,
                                                         consensus: __field1,
                                                         balances: __field2,
                                                         timestamp: __field3,
                                                         session: __field4,
                                                         staking: __field5,
                                                         democracy: __field6,
                                                         council: __field7,
                                                         treasury: __field8,
                                                         contract: __field9,
                                                         tokenbalances:
                                                             __field10,
                                                         financialrecords:
                                                             __field11,
                                                         multisig:
                                                             __field12,})
                    }
                }
                const FIELDS: &'static [&'static str] =
                    &["system", "consensus", "balances", "timestamp",
                      "session", "staking", "democracy", "council",
                      "treasury", "contract", "tokenbalances",
                      "financialrecords", "multisig"];
                _serde::Deserializer::deserialize_struct(__deserializer,
                                                         "GenesisConfig",
                                                         FIELDS,
                                                         __Visitor{marker:
                                                                       _serde::export::PhantomData::<GenesisConfig>,
                                                                   lifetime:
                                                                       _serde::export::PhantomData,})
            }
        }
    };
#[cfg(any(feature = "std", test))]
impl ::BuildStorage for GenesisConfig {
    fn build_storage(self) -> ::std::result::Result<::StorageMap, String> {
        let mut s = ::StorageMap::new();
        if let Some(extra) = self.system { s.extend(extra.build_storage()?); }
        if let Some(extra) = self.consensus {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.balances {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.timestamp {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.session {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.staking {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.democracy {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.council {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.treasury {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.contract {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.tokenbalances {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.financialrecords {
            s.extend(extra.build_storage()?);
        }
        if let Some(extra) = self.multisig {
            s.extend(extra.build_storage()?);
        }
        Ok(s)
    }
}
/// The address format for describing accounts.
pub type Address = balances::Address<Runtime>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic
    =
    generic::UncheckedMortalExtrinsic<Address, Index, Call, Signature>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Index, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive
    =
    executive::Executive<Runtime, Block, balances::ChainContext<Runtime>,
                         Balances, AllModules>;
pub type Symbol = [u8; 8];
pub type TokenDesc = [u8; 32];
pub type TokenBalance = u128;
pub type Precision = u32;
pub mod api {
    /// Dispatch logic for the native runtime.
    pub fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        match method {
            "version" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"version",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output = (|()| super::VERSION)(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "authorities" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"authorities",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|()| super::Consensus::authorities())(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "initialise_block" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"initialise_block",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|header|
                             super::Executive::initialise_block(&header))(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "apply_extrinsic" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"apply_extrinsic",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|extrinsic|
                             super::Executive::apply_extrinsic(extrinsic))(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "execute_block" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"execute_block",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|block|
                             super::Executive::execute_block(block))(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "finalise_block" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"finalise_block",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|()| super::Executive::finalise_block())(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "inherent_extrinsics" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"inherent_extrinsics",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|inherent|
                             super::inherent_extrinsics(inherent))(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "validator_count" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"validator_count",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|()| super::Session::validator_count())(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "validators" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"validators",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output = (|()| super::Session::validators())(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "stake_weight" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"stake_weight",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|account|
                             super::Staking::stake_weight(&account))(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "timestamp" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"timestamp",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output = (|()| super::Timestamp::get())(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "random_seed" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"random_seed",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output = (|()| super::System::random_seed())(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "account_nonce" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"account_nonce",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|account|
                             super::System::account_nonce(&account))(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            "lookup_address" => {
                {
                    let mut data = data;
                    let input =
                        match ::codec::Decode::decode(&mut data) {
                            Some(input) => input,
                            None => {
                                ::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1_formatted(&["Bad input data provided to "],
                                                                                               &match (&"lookup_address",)
                                                                                                    {
                                                                                                    (arg0,)
                                                                                                    =>
                                                                                                    [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                                 ::std::fmt::Display::fmt)],
                                                                                                },
                                                                                               &[::std::fmt::rt::v1::Argument{position:
                                                                                                                                  ::std::fmt::rt::v1::Position::At(0usize),
                                                                                                                              format:
                                                                                                                                  ::std::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                                     ' ',
                                                                                                                                                                 align:
                                                                                                                                                                     ::std::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                                 flags:
                                                                                                                                                                     0u32,
                                                                                                                                                                 precision:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,
                                                                                                                                                                 width:
                                                                                                                                                                     ::std::fmt::rt::v1::Count::Implied,},}]),
                                                      &("runtime/src/lib.rs",
                                                        271u32, 5u32))
                            }
                        };
                    let output =
                        (|address|
                             super::Balances::lookup_address(address))(input);
                    Some(::codec::Encode::encode(&output))
                }
            }
            _ => None,
        }
    }
}
