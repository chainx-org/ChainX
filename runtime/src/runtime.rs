#![feature(prelude_import)]
#![no_std]
// Copyright 2018-2019 Chainpool.

//! The ChainX runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std as std;
mod fee {

    // substrate

    // chainx

    // chainx

    // xrml trait

    // fees
    // assets

    // bridge

    // chainx runtime module
    // fee
    // assets
    // mining
    // dex
    // bridge
    // multisig

    use crate::Call;
    use xassets::Call as XAssetsCall;
    use xbitcoin::Call as XBitcoinCall;
    use xbridge_features::Call as XBridgeFeaturesCall;
    use xfee_manager::SwitchStore;
    use xmultisig::Call as XMultiSigCall;
    use xprocess::Call as XAssetsProcessCall;
    use xsdot::Call as SdotCall;
    use xspot::Call as XSpotCall;
    use xstaking::Call as XStakingCall;
    use xtokens::Call as XTokensCall;
    pub trait CheckFee {
        fn check_fee(&self, switch: SwitchStore) -> Option<u64>;
    }
    impl CheckFee for Call {
        /// Return fee_power, which is part of the total_fee.
        /// total_fee = base_fee * fee_power + byte_fee * bytes
        ///
        /// fee_power = power_per_call
        fn check_fee(&self, switch: SwitchStore) -> Option<u64> {
            let first_check = match self {
                Call::XMultiSig(call) => match call {
                    XMultiSigCall::execute(_, _) => Some(50),
                    XMultiSigCall::confirm(_, _) => Some(25),
                    XMultiSigCall::remove_multi_sig_for(_, _) => Some(1000),
                    _ => None,
                },
                _ => None,
            };
            if first_check.is_some() {
                return first_check;
            }
            if switch.global {
                return None;
            };
            let base_power = match self {
                Call::XAssets(call) => match call {
                    XAssetsCall::transfer(_, _, _, _) => Some(1),
                    _ => None,
                },
                Call::XAssetsProcess(call) => match call {
                    XAssetsProcessCall::withdraw(_, _, _, _) => Some(3),
                    XAssetsProcessCall::revoke_withdraw(_) => Some(10),
                    _ => None,
                },
                Call::XBridgeOfBTC(call) => {
                    let power = if switch.xbtc {
                        None
                    } else {
                        match call {
                            XBitcoinCall::push_header(_) => Some(10),
                            XBitcoinCall::push_transaction(_) => Some(8),
                            XBitcoinCall::create_withdraw_tx(_, _) => Some(5),
                            XBitcoinCall::sign_withdraw_tx(_) => Some(5),
                            _ => None,
                        }
                    };
                    power
                }
                Call::XBridgeOfSDOT(call) => {
                    let power = if switch.sdot {
                        None
                    } else {
                        match call {
                            SdotCall::claim(_, _, _) => Some(2),
                            _ => None,
                        }
                    };
                    power
                }
                Call::XBridgeFeatures(call) => match call {
                    XBridgeFeaturesCall::setup_bitcoin_trustee(_, _, _) => Some(1000),
                    _ => None,
                },
                Call::XStaking(call) => match call {
                    XStakingCall::register(_) => Some(1000000),
                    XStakingCall::refresh(_, _, _, _) => Some(1000),
                    XStakingCall::nominate(_, _, _) => Some(5),
                    XStakingCall::unnominate(_, _, _) => Some(3),
                    XStakingCall::renominate(_, _, _, _) => Some(8),
                    XStakingCall::unfreeze(_, _) => Some(2),
                    XStakingCall::claim(_) => Some(30),
                    _ => None,
                },
                Call::XTokens(call) => match call {
                    XTokensCall::claim(_) => Some(3),
                    _ => None,
                },
                Call::XSpot(call) => {
                    let power = if switch.spot {
                        None
                    } else {
                        match call {
                            XSpotCall::put_order(_, _, _, _, _) => Some(8),
                            XSpotCall::cancel_order(_, _) => Some(2),
                            _ => None,
                        }
                    };
                    power
                }
                _ => None,
            };
            base_power
        }
    }
}
mod trustee {
    use super::{AccountId, Call};
    use support::dispatch::{Dispatchable, Result};
    use system;
    use xbitcoin::Call as XBitcoinCall;
    use xbridge_features::Call as XBridgeFeaturesCall;
    use xmultisig::TrusteeCall;
    use xsupport::{error, info};
    impl TrusteeCall<AccountId> for Call {
        fn allow(&self) -> bool {
            match self {
                Call::XBridgeOfBTC(call) => match call {
                    XBitcoinCall::set_btc_withdrawal_fee_by_trustees(_) => true,
                    XBitcoinCall::fix_withdrawal_state_by_trustees(_, _) => true,
                    _ => false,
                },
                Call::XBridgeFeatures(call) => match call {
                    XBridgeFeaturesCall::transition_trustee_session(_, _) => true,
                    _ => false,
                },
                _ => false,
            }
        }
        fn exec(&self, exerciser: &AccountId) -> Result {
            if !self.allow() {
                {
                    let lvl = ::log::Level::Error;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api_log(
                            ::std::fmt::Arguments::new_v1(
                                &["[runtime|", "|", "L] "],
                                &match (
                                    &"chainx_runtime::trustee",
                                    &31u32,
                                    &::alloc::fmt::format(::std::fmt::Arguments::new_v1(
                                        &["[TrusteeCall]|"],
                                        &match () {
                                            () => [],
                                        },
                                    )),
                                ) {
                                    (arg0, arg1, arg2) => [
                                        ::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt),
                                        ::std::fmt::ArgumentV1::new(arg1, ::std::fmt::Display::fmt),
                                        ::std::fmt::ArgumentV1::new(arg2, ::std::fmt::Display::fmt),
                                    ],
                                },
                            ),
                            lvl,
                            &(
                                "runtime",
                                "chainx_runtime::trustee",
                                "runtime/src/trustee.rs",
                                31u32,
                            ),
                        );
                    }
                };
                return Err("not allow to exec this call for trustee role now");
            }
            {
                let lvl = ::log::Level::Info;
                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                    ::log::__private_api_log(
                        ::std::fmt::Arguments::new_v1(
                            &["[runtime|", "] "],
                            &match (
                                &"chainx_runtime::trustee",
                                &::alloc::fmt::format(::std::fmt::Arguments::new_v1(
                                    &["trustee exec|try to exec from multisig addr:"],
                                    &match (&exerciser,) {
                                        (arg0,) => [::std::fmt::ArgumentV1::new(
                                            arg0,
                                            ::std::fmt::Debug::fmt,
                                        )],
                                    },
                                )),
                            ) {
                                (arg0, arg1) => [
                                    ::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt),
                                    ::std::fmt::ArgumentV1::new(arg1, ::std::fmt::Display::fmt),
                                ],
                            },
                        ),
                        lvl,
                        &(
                            "runtime",
                            "chainx_runtime::trustee",
                            "runtime/src/trustee.rs",
                            34u32,
                        ),
                    );
                }
            };
            let origin = system::RawOrigin::Signed(exerciser.clone()).into();
            if let Err(e) = self.clone().dispatch(origin) {
                if e == "bad origin: expected to be a root origin" {
                    {
                        let lvl = ::log::Level::Info;
                        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                            ::log::__private_api_log(::std::fmt::Arguments::new_v1(&["[runtime|",
                                                                                     "] "],
                                                                                   &match (&"chainx_runtime::trustee",
                                                                                           &::alloc::fmt::format(::std::fmt::Arguments::new_v1(&["failed by executing from addr, try to use root to exec it"],
                                                                                                                                               &match ()
                                                                                                                                                    {
                                                                                                                                                    ()
                                                                                                                                                    =>
                                                                                                                                                    [],
                                                                                                                                                })))
                                                                                        {
                                                                                        (arg0,
                                                                                         arg1)
                                                                                        =>
                                                                                        [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                     ::std::fmt::Display::fmt),
                                                                                         ::std::fmt::ArgumentV1::new(arg1,
                                                                                                                     ::std::fmt::Display::fmt)],
                                                                                    }),
                                                     lvl,
                                                     &("runtime",
                                                       "chainx_runtime::trustee",
                                                       "runtime/src/trustee.rs",
                                                       41u32));
                        }
                    };
                    let origin = system::RawOrigin::Root.into();
                    return self.clone().dispatch(origin);
                }
                return Err(e);
            }
            Ok(())
        }
    }
}
mod xexecutive {
    //! Executive: Handles all of the top-level stuff; essentially just executing blocks/extrinsics.
    use crate::fee::CheckFee;
    use parity_codec::{Codec, Encode};
    use rstd::marker::PhantomData;
    use rstd::prelude::*;
    use rstd::result;
    use runtime_io;
    use runtime_primitives::traits::{
        self, Applyable, As, Block as BlockT, CheckEqual, Checkable, Digest, Header, NumberFor,
        OffchainWorker, OnFinalize, OnInitialize, One, Zero,
    };
    use runtime_primitives::transaction_validity::{
        TransactionLongevity, TransactionPriority, TransactionValidity,
    };
    use runtime_primitives::{ApplyError, ApplyOutcome};
    use support::Dispatchable;
    use system::extrinsics_root;
    use xfee_manager::MakePayment;
    use xr_primitives::traits::Accelerable;
    mod internal {
        pub const MAX_TRANSACTIONS_SIZE: u32 = 4 * 1024 * 1024;
        pub enum ApplyError {
            BadSignature(&'static str),
            Stale,
            Future,
            CantPay,
            FullBlock,
            NotAllow,
        }
        pub enum ApplyOutcome {
            Success,
            Fail(&'static str),
        }
    }
    /// Something that can be used to execute a block.
    pub trait ExecuteBlock<Block: BlockT> {
        /// Actually execute all transitioning for `block`.
        fn execute_block(block: Block);
    }
    pub struct Executive<System, Block, Context, Payment, AllModules>(
        PhantomData<(System, Block, Context, Payment, AllModules)>,
    );
    impl <System: system::Trait + xfee_manager::Trait,
          Block: traits::Block<Header = System::Header, Hash = System::Hash>,
          Context: Default, Payment: MakePayment<System::AccountId>,
          AllModules: OnInitialize<System::BlockNumber> +
          OnFinalize<System::BlockNumber> +
          OffchainWorker<System::BlockNumber>> ExecuteBlock<Block> for
     Executive<System, Block, Context, Payment, AllModules> where
     Block::Extrinsic: Checkable<Context> + Codec,
     <Block::Extrinsic as Checkable<Context>>::Checked: Applyable<Index =
     System::Index, AccountId = System::AccountId> + Accelerable<Index =
     System::Index, AccountId = System::AccountId>,
     <<Block::Extrinsic as Checkable<Context>>::Checked as
     Applyable>::Call: Dispatchable + CheckFee,
     <<<Block::Extrinsic as Checkable<Context>>::Checked as Applyable>::Call
     as Dispatchable>::Origin: From<Option<System::AccountId>> {
        fn execute_block(block: Block) {
            Executive::<System, Block, Context, Payment,
                        AllModules>::execute_block(block);
        }
    }
    impl <System: system::Trait + xfee_manager::Trait,
          Block: traits::Block<Header = System::Header, Hash = System::Hash>,
          Context: Default, Payment: MakePayment<System::AccountId>,
          AllModules: OnInitialize<System::BlockNumber> +
          OnFinalize<System::BlockNumber> +
          OffchainWorker<System::BlockNumber>>
     Executive<System, Block, Context, Payment, AllModules> where
     Block::Extrinsic: Checkable<Context> + Codec,
     <Block::Extrinsic as Checkable<Context>>::Checked: Applyable<Index =
     System::Index, AccountId = System::AccountId> + Accelerable<Index =
     System::Index, AccountId = System::AccountId>,
     <<Block::Extrinsic as Checkable<Context>>::Checked as
     Applyable>::Call: Dispatchable + CheckFee,
     <<<Block::Extrinsic as Checkable<Context>>::Checked as Applyable>::Call
     as Dispatchable>::Origin: From<Option<System::AccountId>> {
        /// Start the execution of a particular block.
        pub fn initialize_block(header: &System::Header) {
            Self::initialize_block_impl(header.number(), header.parent_hash(),
                                        header.extrinsics_root());
        }
        fn initialize_block_impl(block_number: &System::BlockNumber,
                                 parent_hash: &System::Hash,
                                 extrinsics_root: &System::Hash) {
            <system::Module<System>>::initialize(block_number, parent_hash,
                                                 extrinsics_root);
            <AllModules as
                OnInitialize<System::BlockNumber>>::on_initialize(*block_number);
        }
        fn initial_checks(block: &Block) {
            let header = block.header();
            let n = header.number().clone();
            if !(n > System::BlockNumber::zero() &&
                     <system::Module<System>>::block_hash(n -
                                                              System::BlockNumber::one())
                         == *header.parent_hash()) {
                {
                    ::std::rt::begin_panic("Parent hash should be valid.",
                                           &("runtime/src/xexecutive.rs",
                                             111u32, 9u32))
                }
            };
            let xts_root =
                extrinsics_root::<System::Hashing, _>(&block.extrinsics());
            header.extrinsics_root().check_equal(&xts_root);
            if !(header.extrinsics_root() == &xts_root) {
                {
                    ::std::rt::begin_panic("Transaction trie root must be valid.",
                                           &("runtime/src/xexecutive.rs",
                                             119u32, 9u32))
                }
            };
        }
        /// Actually execute all transitioning for `block`.
        pub fn execute_block(block: Block) {
            Self::initialize_block(block.header());
            Self::initial_checks(&block);
            let (header, extrinsics) = block.deconstruct();
            Self::execute_extrinsics_with_book_keeping(extrinsics,
                                                       *header.number());
            Self::final_checks(&header);
        }
        /// Execute given extrinsics and take care of post-extrinsics book-keeping
        fn execute_extrinsics_with_book_keeping(extrinsics:
                                                    Vec<Block::Extrinsic>,
                                                block_number:
                                                    NumberFor<Block>) {
            extrinsics.into_iter().for_each(Self::apply_extrinsic_no_note);
            <system::Module<System>>::note_finished_extrinsics();
            <AllModules as
                OnFinalize<System::BlockNumber>>::on_finalize(block_number);
        }
        /// Finalize the block - it is up the caller to ensure that all header fields are valid
        /// except state-root.
        pub fn finalize_block() -> System::Header {
            <system::Module<System>>::note_finished_extrinsics();
            <AllModules as
                OnFinalize<System::BlockNumber>>::on_finalize(<system::Module<System>>::block_number());
            <system::Module<System>>::derive_extrinsics();
            <system::Module<System>>::finalize()
        }
        /// Apply extrinsic outside of the block execution function.
        /// This doesn't attempt to validate anything regarding the block, but it builds a list of uxt
        /// hashes.
        pub fn apply_extrinsic(uxt: Block::Extrinsic)
         -> result::Result<ApplyOutcome, ApplyError> {
            let encoded = uxt.encode();
            let encoded_len = encoded.len();
            match Self::apply_extrinsic_with_len(uxt, encoded_len,
                                                 Some(encoded)) {
                Ok(internal::ApplyOutcome::Success) =>
                Ok(ApplyOutcome::Success),
                Ok(internal::ApplyOutcome::Fail(_)) => Ok(ApplyOutcome::Fail),
                Err(internal::ApplyError::CantPay) =>
                Err(ApplyError::CantPay),
                Err(internal::ApplyError::BadSignature(_)) =>
                Err(ApplyError::BadSignature),
                Err(internal::ApplyError::Stale) => Err(ApplyError::Stale),
                Err(internal::ApplyError::Future) => Err(ApplyError::Future),
                Err(internal::ApplyError::FullBlock) =>
                Err(ApplyError::FullBlock),
                Err(internal::ApplyError::NotAllow) =>
                Err(ApplyError::CantPay),
            }
        }
        /// Apply an extrinsic inside the block execution function.
        fn apply_extrinsic_no_note(uxt: Block::Extrinsic) {
            let l = uxt.encode().len();
            match Self::apply_extrinsic_with_len(uxt, l, None) {
                Ok(internal::ApplyOutcome::Success) => (),
                Ok(internal::ApplyOutcome::Fail(e)) => runtime_io::print(e),
                Err(internal::ApplyError::CantPay) => {
                    ::std::rt::begin_panic("All extrinsics should have sender able to pay their fees",
                                           &("runtime/src/xexecutive.rs",
                                             181u32, 51u32))
                }
                Err(internal::ApplyError::BadSignature(_)) => {
                    ::std::rt::begin_panic("All extrinsics should be properly signed",
                                           &("runtime/src/xexecutive.rs",
                                             182u32, 59u32))
                }
                Err(internal::ApplyError::Stale) |
                Err(internal::ApplyError::Future) => {
                    ::std::rt::begin_panic("All extrinsics should have the correct nonce",
                                           &("runtime/src/xexecutive.rs",
                                             183u32, 85u32))
                }
                Err(internal::ApplyError::FullBlock) => {
                    ::std::rt::begin_panic("Extrinsics should not exceed block limit",
                                           &("runtime/src/xexecutive.rs",
                                             184u32, 53u32))
                }
                Err(internal::ApplyError::NotAllow) => {
                    ::std::rt::begin_panic("Extrinsics should not allow for this call",
                                           &("runtime/src/xexecutive.rs",
                                             185u32, 52u32))
                }
            }
        }
        /// Actually apply an extrinsic given its `encoded_len`; this doesn't note its hash.
        fn apply_extrinsic_with_len(uxt: Block::Extrinsic, encoded_len: usize,
                                    to_note: Option<Vec<u8>>)
         -> result::Result<internal::ApplyOutcome, internal::ApplyError> {
            let xt =
                uxt.check(&Default::default()).map_err(internal::ApplyError::BadSignature)?;
            if <system::Module<System>>::all_extrinsics_len() +
                   encoded_len as u32 > internal::MAX_TRANSACTIONS_SIZE {
                return Err(internal::ApplyError::FullBlock);
            }
            let mut signed_extrinsic = false;
            if let (Some(sender), Some(index)) = (xt.sender(), xt.index()) {
                let expected_index =
                    <system::Module<System>>::account_nonce(sender);
                if index != &expected_index {
                    return Err(if index < &expected_index {
                                   internal::ApplyError::Stale
                               } else { internal::ApplyError::Future });
                }
                signed_extrinsic = true;
            }
            let acc = xt.acceleration();
            let (f, s) = xt.deconstruct();
            if signed_extrinsic {
                let acc = acc.unwrap();
                let switch = <xfee_manager::Module<System>>::switch();
                if let Some(fee_power) = f.check_fee(switch) {
                    Payment::make_payment(&s.clone().unwrap(), encoded_len,
                                          fee_power,
                                          acc.as_() as
                                              u32).map_err(|_|
                                                               internal::ApplyError::CantPay)?;
                    <system::Module<System>>::inc_account_nonce(&s.clone().unwrap());
                } else { return Err(internal::ApplyError::NotAllow); }
            }
            if let Some(encoded) = to_note {
                <system::Module<System>>::note_extrinsic(encoded);
            }
            let r = f.dispatch(s.into());
            <system::Module<System>>::note_applied_extrinsic(&r,
                                                             encoded_len as
                                                                 u32);
            r.map(|_|
                      internal::ApplyOutcome::Success).or_else(|e|
                                                                   match e {
                                                                       runtime_primitives::BLOCK_FULL
                                                                       =>
                                                                       Err(internal::ApplyError::FullBlock),
                                                                       e =>
                                                                       Ok(internal::ApplyOutcome::Fail(e)),
                                                                   })
        }
        fn final_checks(header: &System::Header) {
            let new_header = <system::Module<System>>::finalize();
            {
                match (&(header.digest().logs().len()),
                       &(new_header.digest().logs().len())) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            {
                                ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["assertion failed: `(left == right)`\n  left: `",
                                                                                            "`,\n right: `",
                                                                                            "`: "],
                                                                                          &match (&&*left_val,
                                                                                                  &&*right_val,
                                                                                                  &::std::fmt::Arguments::new_v1(&["Number of digest items must match that calculated."],
                                                                                                                                 &match ()
                                                                                                                                      {
                                                                                                                                      ()
                                                                                                                                      =>
                                                                                                                                      [],
                                                                                                                                  }))
                                                                                               {
                                                                                               (arg0,
                                                                                                arg1,
                                                                                                arg2)
                                                                                               =>
                                                                                               [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                            ::std::fmt::Debug::fmt),
                                                                                                ::std::fmt::ArgumentV1::new(arg1,
                                                                                                                            ::std::fmt::Debug::fmt),
                                                                                                ::std::fmt::ArgumentV1::new(arg2,
                                                                                                                            ::std::fmt::Display::fmt)],
                                                                                           }),
                                                           &("runtime/src/xexecutive.rs",
                                                             251u32, 9u32))
                            }
                        }
                    }
                }
            };
            let items_zip =
                header.digest().logs().iter().zip(new_header.digest().logs().iter());
            for (header_item, computed_item) in items_zip {
                header_item.check_equal(&computed_item);
                if !(header_item == computed_item) {
                    {
                        ::std::rt::begin_panic("Digest item must match that calculated.",
                                               &("runtime/src/xexecutive.rs",
                                                 259u32, 13u32))
                    }
                };
            }
            let storage_root = new_header.state_root();
            header.state_root().check_equal(&storage_root);
            if !(header.state_root() == storage_root) {
                {
                    ::std::rt::begin_panic("Storage root must match that calculated.",
                                           &("runtime/src/xexecutive.rs",
                                             265u32, 9u32))
                }
            };
        }
        /// Check a given transaction for validity. This doesn't execute any
        /// side-effects; it merely checks whether the transaction would panic if it were included or not.
        ///
        /// Changes made to the storage should be discarded.
        pub fn validate_transaction(uxt: Block::Extrinsic)
         -> TransactionValidity {
            const UNKNOWN_ERROR: i8 = -127;
            const MISSING_SENDER: i8 = -20;
            const INVALID_INDEX: i8 = -10;
            const ACC_ERROR: i8 = -30;
            const NOT_ALLOW: i8 = -1;
            let encoded_len = uxt.encode().len();
            let xt =
                match uxt.check(&Default::default()) {
                    Ok(xt) => xt,
                    Err("invalid account index") =>
                    return TransactionValidity::Unknown(INVALID_INDEX),
                    Err(runtime_primitives::BAD_SIGNATURE) =>
                    return TransactionValidity::Invalid(ApplyError::BadSignature
                                                            as i8),
                    Err(_) =>
                    return TransactionValidity::Invalid(UNKNOWN_ERROR),
                };
            match xt.acceleration() {
                Some(acc) => {
                    if acc.is_zero() {
                        return TransactionValidity::Invalid(ACC_ERROR);
                    }
                }
                None => return TransactionValidity::Invalid(ACC_ERROR),
            }
            let valid =
                if let (Some(sender), Some(index), Some(acceleration)) =
                       (xt.sender(), xt.index(), xt.acceleration()) {
                    let expected_index =
                        <system::Module<System>>::account_nonce(sender);
                    if index < &expected_index {
                        return TransactionValidity::Invalid(ApplyError::Stale
                                                                as i8);
                    }
                    let index = *index;
                    let provides =
                        <[_]>::into_vec(box [(sender, index).encode()]);
                    let requires =
                        if expected_index < index {
                            <[_]>::into_vec(box
                                                [(sender,
                                                  index -
                                                      One::one()).encode()])
                        } else { <[_]>::into_vec(box []) };
                    TransactionValidity::Valid{priority:
                                                   acceleration.as_() as
                                                       TransactionPriority,
                                               requires,
                                               provides,
                                               longevity:
                                                   TransactionLongevity::max_value(),}
                } else {
                    TransactionValidity::Invalid(if xt.sender().is_none() {
                                                     MISSING_SENDER
                                                 } else { INVALID_INDEX })
                };
            let acc = xt.acceleration().unwrap();
            let (f, s) = xt.deconstruct();
            let switch = <xfee_manager::Module<System>>::switch();
            if let Some(fee_power) = f.check_fee(switch) {
                if Payment::check_payment(&s.clone().unwrap(), encoded_len,
                                          fee_power,
                                          acc.as_() as u32).is_err() {
                    return TransactionValidity::Invalid(ApplyError::CantPay as
                                                            i8);
                } else { return valid; }
            } else { return TransactionValidity::Invalid(NOT_ALLOW as i8); }
        }
        /// Start an offchain worker and generate extrinsics.
        pub fn offchain_worker(n: System::BlockNumber) {
            <AllModules as
                OffchainWorker<System::BlockNumber>>::generate_extrinsics(n)
        }
    }
}
use chainx_primitives;
use chainx_primitives::{
    Acceleration, AccountId, AccountIndex, AuthorityId, AuthoritySignature, Balance, BlockNumber,
    Hash, Index, Signature, Timestamp as TimestampU64,
};
use client::{
    block_builder::api::{self as block_builder_api, CheckInherentsResult, InherentData},
    impl_runtime_apis, runtime_api as client_api,
};
use parity_codec::Decode;
use rstd::collections::btree_map::BTreeMap;
use rstd::prelude::*;
use runtime_api;
use runtime_primitives::generic;
use runtime_primitives::traits::{
    AuthorityIdFor, BlakeTwo256, Block as BlockT, DigestFor, NumberFor, StaticLookup,
};
use runtime_primitives::transaction_validity::TransactionValidity;
use runtime_primitives::ApplyResult;
#[cfg(any(feature = "std", test))]
pub use runtime_primitives::BuildStorage;
pub use runtime_primitives::{create_runtime_str, Perbill, Permill};
use substrate_primitives::OpaqueMetadata;
pub use support::{construct_runtime, StorageValue};
pub use timestamp::BlockPeriod;
pub use timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
use version::NativeVersion;
use version::RuntimeVersion;
pub use xaccounts;
pub use xassets;
pub use xbitcoin;
pub use xbridge_common;
use xbridge_common::types::{GenericAllSessionInfo, GenericTrusteeIntentionProps};
pub use xbridge_features;
use xgrandpa::fg_primitives::{self, ScheduledChange};
pub use xprocess;
use xr_primitives::AddrStr;
/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: { ::std::borrow::Cow::Borrowed("chainx") },
    impl_name: { ::std::borrow::Cow::Borrowed("chainx-net") },
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
impl xsystem::Trait for Runtime {
    type ValidatorList = Session;
    type Validator = XAccounts;
}
impl xaccounts::Trait for Runtime {
    type DetermineIntentionJackpotAccountId = xaccounts::SimpleAccountIdDeterminator<Runtime>;
}
impl xfee_manager::Trait for Runtime {
    type Event = Event;
}
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
#[structural_match]
#[rustc_copy_clone_marker]
pub struct Runtime;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::clone::Clone for Runtime {
    #[inline]
    fn clone(&self) -> Runtime {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::marker::Copy for Runtime {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::cmp::PartialEq for Runtime {
    #[inline]
    fn eq(&self, other: &Runtime) -> bool {
        match *other {
            Runtime => match *self {
                Runtime => true,
            },
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::cmp::Eq for Runtime {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
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
impl ::srml_support::runtime_primitives::traits::GetNodeBlockType for Runtime {
    type NodeBlock = chainx_primitives::Block;
}
impl ::srml_support::runtime_primitives::traits::GetRuntimeBlockType for Runtime {
    type RuntimeBlock = Block;
}
#[allow(non_camel_case_types)]
#[structural_match]
pub enum Event {
    system(system::Event),
    indices(indices::Event<Runtime>),
    xsession(xsession::Event<Runtime>),
    xgrandpa(xgrandpa::Event<Runtime>),
    xfee_manager(xfee_manager::Event<Runtime>),
    xassets(xassets::Event<Runtime>),
    xrecords(xrecords::Event<Runtime>),
    xstaking(xstaking::Event<Runtime>),
    xtokens(xtokens::Event<Runtime>),
    xspot(xspot::Event<Runtime>),
    xbitcoin(xbitcoin::Event<Runtime>),
    xsdot(xsdot::Event<Runtime>),
    xbridge_features(xbridge_features::Event<Runtime>),
    xmultisig(xmultisig::Event<Runtime>),
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::clone::Clone for Event {
    #[inline]
    fn clone(&self) -> Event {
        match (&*self,) {
            (&Event::system(ref __self_0),) => {
                Event::system(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::indices(ref __self_0),) => {
                Event::indices(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xsession(ref __self_0),) => {
                Event::xsession(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xgrandpa(ref __self_0),) => {
                Event::xgrandpa(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xfee_manager(ref __self_0),) => {
                Event::xfee_manager(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xassets(ref __self_0),) => {
                Event::xassets(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xrecords(ref __self_0),) => {
                Event::xrecords(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xstaking(ref __self_0),) => {
                Event::xstaking(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xtokens(ref __self_0),) => {
                Event::xtokens(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xspot(ref __self_0),) => {
                Event::xspot(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xbitcoin(ref __self_0),) => {
                Event::xbitcoin(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xsdot(ref __self_0),) => {
                Event::xsdot(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xbridge_features(ref __self_0),) => {
                Event::xbridge_features(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Event::xmultisig(ref __self_0),) => {
                Event::xmultisig(::std::clone::Clone::clone(&(*__self_0)))
            }
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
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Event::system(ref __self_0), &Event::system(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::indices(ref __self_0), &Event::indices(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xsession(ref __self_0), &Event::xsession(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xgrandpa(ref __self_0), &Event::xgrandpa(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xfee_manager(ref __self_0), &Event::xfee_manager(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xassets(ref __self_0), &Event::xassets(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xrecords(ref __self_0), &Event::xrecords(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xstaking(ref __self_0), &Event::xstaking(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xtokens(ref __self_0), &Event::xtokens(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xspot(ref __self_0), &Event::xspot(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xbitcoin(ref __self_0), &Event::xbitcoin(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Event::xsdot(ref __self_0), &Event::xsdot(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (
                        &Event::xbridge_features(ref __self_0),
                        &Event::xbridge_features(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (&Event::xmultisig(ref __self_0), &Event::xmultisig(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                false
            }
        }
    }
    #[inline]
    fn ne(&self, other: &Event) -> bool {
        {
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Event::system(ref __self_0), &Event::system(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::indices(ref __self_0), &Event::indices(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xsession(ref __self_0), &Event::xsession(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xgrandpa(ref __self_0), &Event::xgrandpa(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xfee_manager(ref __self_0), &Event::xfee_manager(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xassets(ref __self_0), &Event::xassets(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xrecords(ref __self_0), &Event::xrecords(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xstaking(ref __self_0), &Event::xstaking(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xtokens(ref __self_0), &Event::xtokens(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xspot(ref __self_0), &Event::xspot(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xbitcoin(ref __self_0), &Event::xbitcoin(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Event::xsdot(ref __self_0), &Event::xsdot(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (
                        &Event::xbridge_features(ref __self_0),
                        &Event::xbridge_features(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (&Event::xmultisig(ref __self_0), &Event::xmultisig(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                true
            }
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
            let _: ::std::cmp::AssertParamIsEq<indices::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xsession::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xgrandpa::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xfee_manager::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xassets::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xrecords::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xstaking::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xtokens::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xspot::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xbitcoin::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xsdot::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xbridge_features::Event<Runtime>>;
            let _: ::std::cmp::AssertParamIsEq<xmultisig::Event<Runtime>>;
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_ENCODE_FOR_Event: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate parity_codec as _parity_codec;
    impl _parity_codec::Encode for Event {
        fn encode_to<EncOut: _parity_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                Event::system(ref aa) => {
                    dest.push_byte(0usize as u8);
                    dest.push(aa);
                }
                Event::indices(ref aa) => {
                    dest.push_byte(1usize as u8);
                    dest.push(aa);
                }
                Event::xsession(ref aa) => {
                    dest.push_byte(2usize as u8);
                    dest.push(aa);
                }
                Event::xgrandpa(ref aa) => {
                    dest.push_byte(3usize as u8);
                    dest.push(aa);
                }
                Event::xfee_manager(ref aa) => {
                    dest.push_byte(4usize as u8);
                    dest.push(aa);
                }
                Event::xassets(ref aa) => {
                    dest.push_byte(5usize as u8);
                    dest.push(aa);
                }
                Event::xrecords(ref aa) => {
                    dest.push_byte(6usize as u8);
                    dest.push(aa);
                }
                Event::xstaking(ref aa) => {
                    dest.push_byte(7usize as u8);
                    dest.push(aa);
                }
                Event::xtokens(ref aa) => {
                    dest.push_byte(8usize as u8);
                    dest.push(aa);
                }
                Event::xspot(ref aa) => {
                    dest.push_byte(9usize as u8);
                    dest.push(aa);
                }
                Event::xbitcoin(ref aa) => {
                    dest.push_byte(10usize as u8);
                    dest.push(aa);
                }
                Event::xsdot(ref aa) => {
                    dest.push_byte(11usize as u8);
                    dest.push(aa);
                }
                Event::xbridge_features(ref aa) => {
                    dest.push_byte(12usize as u8);
                    dest.push(aa);
                }
                Event::xmultisig(ref aa) => {
                    dest.push_byte(13usize as u8);
                    dest.push(aa);
                }
                _ => (),
            }
        }
    }
};
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DECODE_FOR_Event: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate parity_codec as _parity_codec;
    impl _parity_codec::Decode for Event {
        fn decode<DecIn: _parity_codec::Input>(input: &mut DecIn) -> Option<Self> {
            match input.read_byte()? {
                x if x == 0usize as u8 => {
                    Some(Event::system(_parity_codec::Decode::decode(input)?))
                }
                x if x == 1usize as u8 => {
                    Some(Event::indices(_parity_codec::Decode::decode(input)?))
                }
                x if x == 2usize as u8 => {
                    Some(Event::xsession(_parity_codec::Decode::decode(input)?))
                }
                x if x == 3usize as u8 => {
                    Some(Event::xgrandpa(_parity_codec::Decode::decode(input)?))
                }
                x if x == 4usize as u8 => {
                    Some(Event::xfee_manager(_parity_codec::Decode::decode(input)?))
                }
                x if x == 5usize as u8 => {
                    Some(Event::xassets(_parity_codec::Decode::decode(input)?))
                }
                x if x == 6usize as u8 => {
                    Some(Event::xrecords(_parity_codec::Decode::decode(input)?))
                }
                x if x == 7usize as u8 => {
                    Some(Event::xstaking(_parity_codec::Decode::decode(input)?))
                }
                x if x == 8usize as u8 => {
                    Some(Event::xtokens(_parity_codec::Decode::decode(input)?))
                }
                x if x == 9usize as u8 => Some(Event::xspot(_parity_codec::Decode::decode(input)?)),
                x if x == 10usize as u8 => {
                    Some(Event::xbitcoin(_parity_codec::Decode::decode(input)?))
                }
                x if x == 11usize as u8 => {
                    Some(Event::xsdot(_parity_codec::Decode::decode(input)?))
                }
                x if x == 12usize as u8 => Some(Event::xbridge_features(
                    _parity_codec::Decode::decode(input)?,
                )),
                x if x == 13usize as u8 => {
                    Some(Event::xmultisig(_parity_codec::Decode::decode(input)?))
                }
                _ => None,
            }
        }
    }
};
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
            (&Event::indices(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("indices");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xsession(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xsession");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xgrandpa(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xgrandpa");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xfee_manager(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xfee_manager");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xassets(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xassets");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xrecords(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xrecords");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xstaking(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xstaking");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xtokens(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xtokens");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xspot(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xspot");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xbitcoin(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xbitcoin");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xsdot(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xsdot");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xbridge_features(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xbridge_features");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Event::xmultisig(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xmultisig");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
        }
    }
}
impl From<system::Event> for Event {
    fn from(x: system::Event) -> Self {
        Event::system(x)
    }
}
impl From<indices::Event<Runtime>> for Event {
    fn from(x: indices::Event<Runtime>) -> Self {
        Event::indices(x)
    }
}
impl From<xsession::Event<Runtime>> for Event {
    fn from(x: xsession::Event<Runtime>) -> Self {
        Event::xsession(x)
    }
}
impl From<xgrandpa::Event<Runtime>> for Event {
    fn from(x: xgrandpa::Event<Runtime>) -> Self {
        Event::xgrandpa(x)
    }
}
impl From<xfee_manager::Event<Runtime>> for Event {
    fn from(x: xfee_manager::Event<Runtime>) -> Self {
        Event::xfee_manager(x)
    }
}
impl From<xassets::Event<Runtime>> for Event {
    fn from(x: xassets::Event<Runtime>) -> Self {
        Event::xassets(x)
    }
}
impl From<xrecords::Event<Runtime>> for Event {
    fn from(x: xrecords::Event<Runtime>) -> Self {
        Event::xrecords(x)
    }
}
impl From<xstaking::Event<Runtime>> for Event {
    fn from(x: xstaking::Event<Runtime>) -> Self {
        Event::xstaking(x)
    }
}
impl From<xtokens::Event<Runtime>> for Event {
    fn from(x: xtokens::Event<Runtime>) -> Self {
        Event::xtokens(x)
    }
}
impl From<xspot::Event<Runtime>> for Event {
    fn from(x: xspot::Event<Runtime>) -> Self {
        Event::xspot(x)
    }
}
impl From<xbitcoin::Event<Runtime>> for Event {
    fn from(x: xbitcoin::Event<Runtime>) -> Self {
        Event::xbitcoin(x)
    }
}
impl From<xsdot::Event<Runtime>> for Event {
    fn from(x: xsdot::Event<Runtime>) -> Self {
        Event::xsdot(x)
    }
}
impl From<xbridge_features::Event<Runtime>> for Event {
    fn from(x: xbridge_features::Event<Runtime>) -> Self {
        Event::xbridge_features(x)
    }
}
impl From<xmultisig::Event<Runtime>> for Event {
    fn from(x: xmultisig::Event<Runtime>) -> Self {
        Event::xmultisig(x)
    }
}
impl Runtime {
    #[allow(dead_code)]
    pub fn outer_event_metadata() -> ::srml_support::event::OuterEventMetadata {
        ::srml_support::event::OuterEventMetadata {
            name: ::srml_support::event::DecodeDifferent::Encode("Event"),
            events: ::srml_support::event::DecodeDifferent::Encode(&[
                (
                    "system",
                    ::srml_support::event::FnEncode(system::Event::metadata),
                ),
                (
                    "indices",
                    ::srml_support::event::FnEncode(indices::Event::<Runtime>::metadata),
                ),
                (
                    "xsession",
                    ::srml_support::event::FnEncode(xsession::Event::<Runtime>::metadata),
                ),
                (
                    "xgrandpa",
                    ::srml_support::event::FnEncode(xgrandpa::Event::<Runtime>::metadata),
                ),
                (
                    "xfee_manager",
                    ::srml_support::event::FnEncode(xfee_manager::Event::<Runtime>::metadata),
                ),
                (
                    "xassets",
                    ::srml_support::event::FnEncode(xassets::Event::<Runtime>::metadata),
                ),
                (
                    "xrecords",
                    ::srml_support::event::FnEncode(xrecords::Event::<Runtime>::metadata),
                ),
                (
                    "xstaking",
                    ::srml_support::event::FnEncode(xstaking::Event::<Runtime>::metadata),
                ),
                (
                    "xtokens",
                    ::srml_support::event::FnEncode(xtokens::Event::<Runtime>::metadata),
                ),
                (
                    "xspot",
                    ::srml_support::event::FnEncode(xspot::Event::<Runtime>::metadata),
                ),
                (
                    "xbitcoin",
                    ::srml_support::event::FnEncode(xbitcoin::Event::<Runtime>::metadata),
                ),
                (
                    "xsdot",
                    ::srml_support::event::FnEncode(xsdot::Event::<Runtime>::metadata),
                ),
                (
                    "xbridge_features",
                    ::srml_support::event::FnEncode(xbridge_features::Event::<Runtime>::metadata),
                ),
                (
                    "xmultisig",
                    ::srml_support::event::FnEncode(xmultisig::Event::<Runtime>::metadata),
                ),
            ]),
        }
    }
    #[allow(dead_code)]
    pub fn __module_events_system() -> &'static [::srml_support::event::EventMetadata] {
        system::Event::metadata()
    }
    pub fn __module_events_indices() -> &'static [::srml_support::event::EventMetadata] {
        indices::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xsession() -> &'static [::srml_support::event::EventMetadata] {
        xsession::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xgrandpa() -> &'static [::srml_support::event::EventMetadata] {
        xgrandpa::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xfee_manager() -> &'static [::srml_support::event::EventMetadata] {
        xfee_manager::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xassets() -> &'static [::srml_support::event::EventMetadata] {
        xassets::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xrecords() -> &'static [::srml_support::event::EventMetadata] {
        xrecords::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xstaking() -> &'static [::srml_support::event::EventMetadata] {
        xstaking::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xtokens() -> &'static [::srml_support::event::EventMetadata] {
        xtokens::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xspot() -> &'static [::srml_support::event::EventMetadata] {
        xspot::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xbitcoin() -> &'static [::srml_support::event::EventMetadata] {
        xbitcoin::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xsdot() -> &'static [::srml_support::event::EventMetadata] {
        xsdot::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xbridge_features() -> &'static [::srml_support::event::EventMetadata] {
        xbridge_features::Event::<Runtime>::metadata()
    }
    pub fn __module_events_xmultisig() -> &'static [::srml_support::event::EventMetadata] {
        xmultisig::Event::<Runtime>::metadata()
    }
}
#[allow(non_camel_case_types)]
#[structural_match]
pub enum Origin {
    system(system::Origin<Runtime>),

    #[allow(dead_code)]
    Void(::srml_support::Void),
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::clone::Clone for Origin {
    #[inline]
    fn clone(&self) -> Origin {
        match (&*self,) {
            (&Origin::system(ref __self_0),) => {
                Origin::system(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Origin::Void(ref __self_0),) => {
                Origin::Void(::std::clone::Clone::clone(&(*__self_0)))
            }
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
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Origin::system(ref __self_0), &Origin::system(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Origin::Void(ref __self_0), &Origin::Void(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                false
            }
        }
    }
    #[inline]
    fn ne(&self, other: &Origin) -> bool {
        {
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Origin::system(ref __self_0), &Origin::system(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Origin::Void(ref __self_0), &Origin::Void(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                true
            }
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
            let _: ::std::cmp::AssertParamIsEq<::srml_support::Void>;
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
    pub const NONE: Self = Origin::system(system::RawOrigin::None);
    pub const ROOT: Self = Origin::system(system::RawOrigin::Root);
    pub fn signed(by: <Runtime as system::Trait>::AccountId) -> Self {
        Origin::system(system::RawOrigin::Signed(by))
    }
}
impl From<system::Origin<Runtime>> for Origin {
    fn from(x: system::Origin<Runtime>) -> Self {
        Origin::system(x)
    }
}
impl Into<Option<system::Origin<Runtime>>> for Origin {
    fn into(self) -> Option<system::Origin<Runtime>> {
        if let Origin::system(l) = self {
            Some(l)
        } else {
            None
        }
    }
}
impl From<Option<<Runtime as system::Trait>::AccountId>> for Origin {
    fn from(x: Option<<Runtime as system::Trait>::AccountId>) -> Self {
        <system::Origin<Runtime>>::from(x).into()
    }
}
pub type System = system::Module<Runtime>;
pub type Indices = indices::Module<Runtime>;
pub type Timestamp = timestamp::Module<Runtime>;
pub type Consensus = consensus::Module<Runtime>;
pub type Session = xsession::Module<Runtime>;
pub type FinalityTracker = finality_tracker::Module<Runtime>;
pub type Grandpa = xgrandpa::Module<Runtime>;
pub type Aura = xaura::Module<Runtime>;
pub type XSystem = xsystem::Module<Runtime>;
pub type XAccounts = xaccounts::Module<Runtime>;
pub type XFeeManager = xfee_manager::Module<Runtime>;
pub type XAssets = xassets::Module<Runtime>;
pub type XAssetsRecords = xrecords::Module<Runtime>;
pub type XAssetsProcess = xprocess::Module<Runtime>;
pub type XStaking = xstaking::Module<Runtime>;
pub type XTokens = xtokens::Module<Runtime>;
pub type XSpot = xspot::Module<Runtime>;
pub type XBridgeOfBTC = xbitcoin::Module<Runtime>;
pub type XBridgeOfSDOT = xsdot::Module<Runtime>;
pub type XBridgeFeatures = xbridge_features::Module<Runtime>;
pub type XMultiSig = xmultisig::Module<Runtime>;
type AllModules = (
    Indices,
    Timestamp,
    Consensus,
    Session,
    FinalityTracker,
    Grandpa,
    Aura,
    XSystem,
    XAccounts,
    XFeeManager,
    XAssets,
    XAssetsRecords,
    XAssetsProcess,
    XStaking,
    XTokens,
    XSpot,
    XBridgeOfBTC,
    XBridgeOfSDOT,
    XBridgeFeatures,
    XMultiSig,
);
#[structural_match]
pub enum Call {
    Indices(::srml_support::dispatch::CallableCallFor<Indices>),
    Timestamp(::srml_support::dispatch::CallableCallFor<Timestamp>),
    Consensus(::srml_support::dispatch::CallableCallFor<Consensus>),
    Session(::srml_support::dispatch::CallableCallFor<Session>),
    FinalityTracker(::srml_support::dispatch::CallableCallFor<FinalityTracker>),
    Grandpa(::srml_support::dispatch::CallableCallFor<Grandpa>),
    XSystem(::srml_support::dispatch::CallableCallFor<XSystem>),
    XFeeManager(::srml_support::dispatch::CallableCallFor<XFeeManager>),
    XAssets(::srml_support::dispatch::CallableCallFor<XAssets>),
    XAssetsRecords(::srml_support::dispatch::CallableCallFor<XAssetsRecords>),
    XAssetsProcess(::srml_support::dispatch::CallableCallFor<XAssetsProcess>),
    XStaking(::srml_support::dispatch::CallableCallFor<XStaking>),
    XTokens(::srml_support::dispatch::CallableCallFor<XTokens>),
    XSpot(::srml_support::dispatch::CallableCallFor<XSpot>),
    XBridgeOfBTC(::srml_support::dispatch::CallableCallFor<XBridgeOfBTC>),
    XBridgeOfSDOT(::srml_support::dispatch::CallableCallFor<XBridgeOfSDOT>),
    XBridgeFeatures(::srml_support::dispatch::CallableCallFor<XBridgeFeatures>),
    XMultiSig(::srml_support::dispatch::CallableCallFor<XMultiSig>),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::clone::Clone for Call {
    #[inline]
    fn clone(&self) -> Call {
        match (&*self,) {
            (&Call::Indices(ref __self_0),) => {
                Call::Indices(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::Timestamp(ref __self_0),) => {
                Call::Timestamp(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::Consensus(ref __self_0),) => {
                Call::Consensus(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::Session(ref __self_0),) => {
                Call::Session(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::FinalityTracker(ref __self_0),) => {
                Call::FinalityTracker(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::Grandpa(ref __self_0),) => {
                Call::Grandpa(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XSystem(ref __self_0),) => {
                Call::XSystem(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XFeeManager(ref __self_0),) => {
                Call::XFeeManager(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XAssets(ref __self_0),) => {
                Call::XAssets(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XAssetsRecords(ref __self_0),) => {
                Call::XAssetsRecords(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XAssetsProcess(ref __self_0),) => {
                Call::XAssetsProcess(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XStaking(ref __self_0),) => {
                Call::XStaking(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XTokens(ref __self_0),) => {
                Call::XTokens(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XSpot(ref __self_0),) => Call::XSpot(::std::clone::Clone::clone(&(*__self_0))),
            (&Call::XBridgeOfBTC(ref __self_0),) => {
                Call::XBridgeOfBTC(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XBridgeOfSDOT(ref __self_0),) => {
                Call::XBridgeOfSDOT(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XBridgeFeatures(ref __self_0),) => {
                Call::XBridgeFeatures(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&Call::XMultiSig(ref __self_0),) => {
                Call::XMultiSig(::std::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::cmp::PartialEq for Call {
    #[inline]
    fn eq(&self, other: &Call) -> bool {
        {
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Call::Indices(ref __self_0), &Call::Indices(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::Timestamp(ref __self_0), &Call::Timestamp(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::Consensus(ref __self_0), &Call::Consensus(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::Session(ref __self_0), &Call::Session(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (
                        &Call::FinalityTracker(ref __self_0),
                        &Call::FinalityTracker(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (&Call::Grandpa(ref __self_0), &Call::Grandpa(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XSystem(ref __self_0), &Call::XSystem(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XFeeManager(ref __self_0), &Call::XFeeManager(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XAssets(ref __self_0), &Call::XAssets(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XAssetsRecords(ref __self_0), &Call::XAssetsRecords(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XAssetsProcess(ref __self_0), &Call::XAssetsProcess(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XStaking(ref __self_0), &Call::XStaking(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XTokens(ref __self_0), &Call::XTokens(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XSpot(ref __self_0), &Call::XSpot(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XBridgeOfBTC(ref __self_0), &Call::XBridgeOfBTC(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (&Call::XBridgeOfSDOT(ref __self_0), &Call::XBridgeOfSDOT(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (
                        &Call::XBridgeFeatures(ref __self_0),
                        &Call::XBridgeFeatures(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (&Call::XMultiSig(ref __self_0), &Call::XMultiSig(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                false
            }
        }
    }
    #[inline]
    fn ne(&self, other: &Call) -> bool {
        {
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&Call::Indices(ref __self_0), &Call::Indices(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::Timestamp(ref __self_0), &Call::Timestamp(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::Consensus(ref __self_0), &Call::Consensus(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::Session(ref __self_0), &Call::Session(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (
                        &Call::FinalityTracker(ref __self_0),
                        &Call::FinalityTracker(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (&Call::Grandpa(ref __self_0), &Call::Grandpa(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XSystem(ref __self_0), &Call::XSystem(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XFeeManager(ref __self_0), &Call::XFeeManager(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XAssets(ref __self_0), &Call::XAssets(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XAssetsRecords(ref __self_0), &Call::XAssetsRecords(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XAssetsProcess(ref __self_0), &Call::XAssetsProcess(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XStaking(ref __self_0), &Call::XStaking(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XTokens(ref __self_0), &Call::XTokens(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XSpot(ref __self_0), &Call::XSpot(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XBridgeOfBTC(ref __self_0), &Call::XBridgeOfBTC(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (&Call::XBridgeOfSDOT(ref __self_0), &Call::XBridgeOfSDOT(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (
                        &Call::XBridgeFeatures(ref __self_0),
                        &Call::XBridgeFeatures(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (&Call::XMultiSig(ref __self_0), &Call::XMultiSig(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                true
            }
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
            let _: ::std::cmp::AssertParamIsEq<::srml_support::dispatch::CallableCallFor<Indices>>;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<Timestamp>,
            >;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<Consensus>,
            >;
            let _: ::std::cmp::AssertParamIsEq<::srml_support::dispatch::CallableCallFor<Session>>;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<FinalityTracker>,
            >;
            let _: ::std::cmp::AssertParamIsEq<::srml_support::dispatch::CallableCallFor<Grandpa>>;
            let _: ::std::cmp::AssertParamIsEq<::srml_support::dispatch::CallableCallFor<XSystem>>;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XFeeManager>,
            >;
            let _: ::std::cmp::AssertParamIsEq<::srml_support::dispatch::CallableCallFor<XAssets>>;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XAssetsRecords>,
            >;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XAssetsProcess>,
            >;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XStaking>,
            >;
            let _: ::std::cmp::AssertParamIsEq<::srml_support::dispatch::CallableCallFor<XTokens>>;
            let _: ::std::cmp::AssertParamIsEq<::srml_support::dispatch::CallableCallFor<XSpot>>;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XBridgeOfBTC>,
            >;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XBridgeOfSDOT>,
            >;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XBridgeFeatures>,
            >;
            let _: ::std::cmp::AssertParamIsEq<
                ::srml_support::dispatch::CallableCallFor<XMultiSig>,
            >;
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_ENCODE_FOR_Call: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate parity_codec as _parity_codec;
    impl _parity_codec::Encode for Call {
        fn encode_to<EncOut: _parity_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                Call::Indices(ref aa) => {
                    dest.push_byte(0usize as u8);
                    dest.push(aa);
                }
                Call::Timestamp(ref aa) => {
                    dest.push_byte(1usize as u8);
                    dest.push(aa);
                }
                Call::Consensus(ref aa) => {
                    dest.push_byte(2usize as u8);
                    dest.push(aa);
                }
                Call::Session(ref aa) => {
                    dest.push_byte(3usize as u8);
                    dest.push(aa);
                }
                Call::FinalityTracker(ref aa) => {
                    dest.push_byte(4usize as u8);
                    dest.push(aa);
                }
                Call::Grandpa(ref aa) => {
                    dest.push_byte(5usize as u8);
                    dest.push(aa);
                }
                Call::XSystem(ref aa) => {
                    dest.push_byte(6usize as u8);
                    dest.push(aa);
                }
                Call::XFeeManager(ref aa) => {
                    dest.push_byte(7usize as u8);
                    dest.push(aa);
                }
                Call::XAssets(ref aa) => {
                    dest.push_byte(8usize as u8);
                    dest.push(aa);
                }
                Call::XAssetsRecords(ref aa) => {
                    dest.push_byte(9usize as u8);
                    dest.push(aa);
                }
                Call::XAssetsProcess(ref aa) => {
                    dest.push_byte(10usize as u8);
                    dest.push(aa);
                }
                Call::XStaking(ref aa) => {
                    dest.push_byte(11usize as u8);
                    dest.push(aa);
                }
                Call::XTokens(ref aa) => {
                    dest.push_byte(12usize as u8);
                    dest.push(aa);
                }
                Call::XSpot(ref aa) => {
                    dest.push_byte(13usize as u8);
                    dest.push(aa);
                }
                Call::XBridgeOfBTC(ref aa) => {
                    dest.push_byte(14usize as u8);
                    dest.push(aa);
                }
                Call::XBridgeOfSDOT(ref aa) => {
                    dest.push_byte(15usize as u8);
                    dest.push(aa);
                }
                Call::XBridgeFeatures(ref aa) => {
                    dest.push_byte(16usize as u8);
                    dest.push(aa);
                }
                Call::XMultiSig(ref aa) => {
                    dest.push_byte(17usize as u8);
                    dest.push(aa);
                }
                _ => (),
            }
        }
    }
};
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DECODE_FOR_Call: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate parity_codec as _parity_codec;
    impl _parity_codec::Decode for Call {
        fn decode<DecIn: _parity_codec::Input>(input: &mut DecIn) -> Option<Self> {
            match input.read_byte()? {
                x if x == 0usize as u8 => {
                    Some(Call::Indices(_parity_codec::Decode::decode(input)?))
                }
                x if x == 1usize as u8 => {
                    Some(Call::Timestamp(_parity_codec::Decode::decode(input)?))
                }
                x if x == 2usize as u8 => {
                    Some(Call::Consensus(_parity_codec::Decode::decode(input)?))
                }
                x if x == 3usize as u8 => {
                    Some(Call::Session(_parity_codec::Decode::decode(input)?))
                }
                x if x == 4usize as u8 => {
                    Some(Call::FinalityTracker(_parity_codec::Decode::decode(input)?))
                }
                x if x == 5usize as u8 => {
                    Some(Call::Grandpa(_parity_codec::Decode::decode(input)?))
                }
                x if x == 6usize as u8 => {
                    Some(Call::XSystem(_parity_codec::Decode::decode(input)?))
                }
                x if x == 7usize as u8 => {
                    Some(Call::XFeeManager(_parity_codec::Decode::decode(input)?))
                }
                x if x == 8usize as u8 => {
                    Some(Call::XAssets(_parity_codec::Decode::decode(input)?))
                }
                x if x == 9usize as u8 => {
                    Some(Call::XAssetsRecords(_parity_codec::Decode::decode(input)?))
                }
                x if x == 10usize as u8 => {
                    Some(Call::XAssetsProcess(_parity_codec::Decode::decode(input)?))
                }
                x if x == 11usize as u8 => {
                    Some(Call::XStaking(_parity_codec::Decode::decode(input)?))
                }
                x if x == 12usize as u8 => {
                    Some(Call::XTokens(_parity_codec::Decode::decode(input)?))
                }
                x if x == 13usize as u8 => Some(Call::XSpot(_parity_codec::Decode::decode(input)?)),
                x if x == 14usize as u8 => {
                    Some(Call::XBridgeOfBTC(_parity_codec::Decode::decode(input)?))
                }
                x if x == 15usize as u8 => {
                    Some(Call::XBridgeOfSDOT(_parity_codec::Decode::decode(input)?))
                }
                x if x == 16usize as u8 => {
                    Some(Call::XBridgeFeatures(_parity_codec::Decode::decode(input)?))
                }
                x if x == 17usize as u8 => {
                    Some(Call::XMultiSig(_parity_codec::Decode::decode(input)?))
                }
                _ => None,
            }
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::fmt::Debug for Call {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match (&*self,) {
            (&Call::Indices(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Indices");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Timestamp(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Timestamp");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Consensus(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Consensus");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Session(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Session");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::FinalityTracker(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("FinalityTracker");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::Grandpa(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Grandpa");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XSystem(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XSystem");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XFeeManager(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XFeeManager");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XAssets(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XAssets");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XAssetsRecords(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XAssetsRecords");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XAssetsProcess(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XAssetsProcess");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XStaking(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XStaking");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XTokens(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XTokens");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XSpot(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XSpot");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XBridgeOfBTC(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XBridgeOfBTC");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XBridgeOfSDOT(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XBridgeOfSDOT");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XBridgeFeatures(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XBridgeFeatures");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Call::XMultiSig(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("XMultiSig");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
        }
    }
}
impl ::srml_support::dispatch::Dispatchable for Call {
    type Origin = Origin;
    type Trait = Call;
    fn dispatch(self, origin: Origin) -> ::srml_support::dispatch::Result {
        match self {
            Call::Indices(call) => call.dispatch(origin),
            Call::Timestamp(call) => call.dispatch(origin),
            Call::Consensus(call) => call.dispatch(origin),
            Call::Session(call) => call.dispatch(origin),
            Call::FinalityTracker(call) => call.dispatch(origin),
            Call::Grandpa(call) => call.dispatch(origin),
            Call::XSystem(call) => call.dispatch(origin),
            Call::XFeeManager(call) => call.dispatch(origin),
            Call::XAssets(call) => call.dispatch(origin),
            Call::XAssetsRecords(call) => call.dispatch(origin),
            Call::XAssetsProcess(call) => call.dispatch(origin),
            Call::XStaking(call) => call.dispatch(origin),
            Call::XTokens(call) => call.dispatch(origin),
            Call::XSpot(call) => call.dispatch(origin),
            Call::XBridgeOfBTC(call) => call.dispatch(origin),
            Call::XBridgeOfSDOT(call) => call.dispatch(origin),
            Call::XBridgeFeatures(call) => call.dispatch(origin),
            Call::XMultiSig(call) => call.dispatch(origin),
        }
    }
}
impl ::srml_support::dispatch::IsSubType<Indices> for Call {
    fn is_aux_sub_type(&self) -> Option<&<Indices as ::srml_support::dispatch::Callable>::Call> {
        if let Call::Indices(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<Timestamp> for Call {
    fn is_aux_sub_type(&self) -> Option<&<Timestamp as ::srml_support::dispatch::Callable>::Call> {
        if let Call::Timestamp(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<Consensus> for Call {
    fn is_aux_sub_type(&self) -> Option<&<Consensus as ::srml_support::dispatch::Callable>::Call> {
        if let Call::Consensus(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<Session> for Call {
    fn is_aux_sub_type(&self) -> Option<&<Session as ::srml_support::dispatch::Callable>::Call> {
        if let Call::Session(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<FinalityTracker> for Call {
    fn is_aux_sub_type(
        &self,
    ) -> Option<&<FinalityTracker as ::srml_support::dispatch::Callable>::Call> {
        if let Call::FinalityTracker(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<Grandpa> for Call {
    fn is_aux_sub_type(&self) -> Option<&<Grandpa as ::srml_support::dispatch::Callable>::Call> {
        if let Call::Grandpa(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XSystem> for Call {
    fn is_aux_sub_type(&self) -> Option<&<XSystem as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XSystem(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XFeeManager> for Call {
    fn is_aux_sub_type(
        &self,
    ) -> Option<&<XFeeManager as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XFeeManager(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XAssets> for Call {
    fn is_aux_sub_type(&self) -> Option<&<XAssets as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XAssets(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XAssetsRecords> for Call {
    fn is_aux_sub_type(
        &self,
    ) -> Option<&<XAssetsRecords as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XAssetsRecords(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XAssetsProcess> for Call {
    fn is_aux_sub_type(
        &self,
    ) -> Option<&<XAssetsProcess as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XAssetsProcess(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XStaking> for Call {
    fn is_aux_sub_type(&self) -> Option<&<XStaking as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XStaking(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XTokens> for Call {
    fn is_aux_sub_type(&self) -> Option<&<XTokens as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XTokens(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XSpot> for Call {
    fn is_aux_sub_type(&self) -> Option<&<XSpot as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XSpot(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XBridgeOfBTC> for Call {
    fn is_aux_sub_type(
        &self,
    ) -> Option<&<XBridgeOfBTC as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XBridgeOfBTC(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XBridgeOfSDOT> for Call {
    fn is_aux_sub_type(
        &self,
    ) -> Option<&<XBridgeOfSDOT as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XBridgeOfSDOT(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XBridgeFeatures> for Call {
    fn is_aux_sub_type(
        &self,
    ) -> Option<&<XBridgeFeatures as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XBridgeFeatures(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl ::srml_support::dispatch::IsSubType<XMultiSig> for Call {
    fn is_aux_sub_type(&self) -> Option<&<XMultiSig as ::srml_support::dispatch::Callable>::Call> {
        if let Call::XMultiSig(ref r) = *self {
            Some(r)
        } else {
            None
        }
    }
}
impl Runtime {
    pub fn metadata() -> ::srml_support::metadata::RuntimeMetadataPrefixed {
        ::srml_support::metadata::RuntimeMetadata::V4(::srml_support::metadata::RuntimeMetadataV4{modules:
                                                                                                      ::srml_support::metadata::DecodeDifferent::Encode(&[::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("system"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(system::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(system::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       None,
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ system > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_system
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_system
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("indices"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(indices::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(indices::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(indices::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ indices > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_indices
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_indices
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("timestamp"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(timestamp::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(timestamp::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(timestamp::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       None,},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("consensus"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(consensus::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(consensus::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(consensus::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       None,},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xsession"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsession::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsession::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsession::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xsession > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xsession
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xsession
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("finality_tracker"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(||
                                                                                                                                                                                                                                                                                                "")),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       None,
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(finality_tracker::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       None,},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xgrandpa"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xgrandpa::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xgrandpa::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xgrandpa::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xgrandpa > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xgrandpa
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xgrandpa
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xaura"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(||
                                                                                                                                                                                                                                                                                                "")),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       None,
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       None,
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       None,},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xsystem"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsystem::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsystem::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsystem::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       None,},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xaccounts"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(||
                                                                                                                                                                                                                                                                                                "")),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       None,
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       None,
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       None,},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xfee_manager"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xfee_manager::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xfee_manager::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xfee_manager::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xfee_manager > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xfee_manager
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xfee_manager
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xassets"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xassets::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xassets::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xassets::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xassets > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xassets
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xassets
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xrecords"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xrecords::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xrecords::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xrecords::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xrecords > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xrecords
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xrecords
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xprocess"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xprocess::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xprocess::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xprocess::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       None,},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xstaking"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xstaking::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xstaking::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xstaking::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xstaking > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xstaking
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xstaking
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xtokens"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xtokens::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xtokens::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xtokens::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xtokens > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xtokens
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xtokens
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xspot"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xspot::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xspot::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xspot::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xspot > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xspot
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xspot
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xbitcoin"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xbitcoin::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xbitcoin::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xbitcoin::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xbitcoin > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xbitcoin
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xbitcoin
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xsdot"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsdot::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsdot::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xsdot::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xsdot > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xsdot
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xsdot
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xbridge_features"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xbridge_features::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xbridge_features::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xbridge_features::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xbridge_features > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xbridge_features
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xbridge_features
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),},
                                                                                                                                                          ::srml_support::metadata::ModuleMetadata{name:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode("xmultisig"),
                                                                                                                                                                                                   prefix:
                                                                                                                                                                                                       ::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xmultisig::Module::<Runtime>::store_metadata_name)),
                                                                                                                                                                                                   storage:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xmultisig::Module::<Runtime>::store_metadata_functions))),
                                                                                                                                                                                                   calls:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode(xmultisig::Module::<Runtime>::call_functions))),
                                                                                                                                                                                                   event:
                                                                                                                                                                                                       Some(::srml_support::metadata::DecodeDifferent::Encode(::srml_support::metadata::FnEncode({
                                                                                                                                                                                                                                                                                                     enum ProcMacroHack
                                                                                                                                                                                                                                                                                                          {
                                                                                                                                                                                                                                                                                                         Value
                                                                                                                                                                                                                                                                                                             =
                                                                                                                                                                                                                                                                                                             ("Runtime :: [ < __module_events_ xmultisig > ]",
                                                                                                                                                                                                                                                                                                              0).1,
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                     macro_rules! proc_macro_call((

                                                                                                                                                                                                                                                                                                                                  )
                                                                                                                                                                                                                                                                                                                                  =>
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  {
                                                                                                                                                                                                                                                                                                                                  Runtime
                                                                                                                                                                                                                                                                                                                                  ::
                                                                                                                                                                                                                                                                                                                                  __module_events_xmultisig
                                                                                                                                                                                                                                                                                                                                  }
                                                                                                                                                                                                                                                                                                                                  });
                                                                                                                                                                                                                                                                                                     {
                                                                                                                                                                                                                                                                                                         Runtime::__module_events_xmultisig
                                                                                                                                                                                                                                                                                                     }
                                                                                                                                                                                                                                                                                                 }))),}]),}).into()
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
            Log(ref __self_0_0) => Log(::std::clone::Clone::clone(&(*__self_0_0))),
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
            Log(ref __self_1_0) => match *self {
                Log(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Log) -> bool {
        match *other {
            Log(ref __self_1_0) => match *self {
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
        {
            let _: ::std::cmp::AssertParamIsEq<InternalLog>;
        }
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
const _IMPL_SERIALIZE_FOR_Log: () = {
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
        fn serialize<__S>(&self, __serializer: __S) -> _serde::export::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            _serde::Serializer::serialize_newtype_struct(__serializer, "Log", &self.0)
        }
    }
};
/// All possible log entries for the `$trait` runtime. `Encode`/`Decode` implementations
/// are auto-generated => it is not binary-compatible with `generic::DigestItem`.
#[allow(non_camel_case_types)]
#[structural_match]
pub enum InternalLog {
    system(system::Log<Runtime>),
    consensus(consensus::Log<Runtime>),
    xgrandpa(xgrandpa::Log<Runtime>),
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_camel_case_types)]
impl ::std::clone::Clone for InternalLog {
    #[inline]
    fn clone(&self) -> InternalLog {
        match (&*self,) {
            (&InternalLog::system(ref __self_0),) => {
                InternalLog::system(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&InternalLog::consensus(ref __self_0),) => {
                InternalLog::consensus(::std::clone::Clone::clone(&(*__self_0)))
            }
            (&InternalLog::xgrandpa(ref __self_0),) => {
                InternalLog::xgrandpa(::std::clone::Clone::clone(&(*__self_0)))
            }
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
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&InternalLog::system(ref __self_0), &InternalLog::system(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (
                        &InternalLog::consensus(ref __self_0),
                        &InternalLog::consensus(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (
                        &InternalLog::xgrandpa(ref __self_0),
                        &InternalLog::xgrandpa(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                false
            }
        }
    }
    #[inline]
    fn ne(&self, other: &InternalLog) -> bool {
        {
            let __self_vi = unsafe { ::std::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::std::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (&InternalLog::system(ref __self_0), &InternalLog::system(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (
                        &InternalLog::consensus(ref __self_0),
                        &InternalLog::consensus(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (
                        &InternalLog::xgrandpa(ref __self_0),
                        &InternalLog::xgrandpa(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    _ => unsafe { ::std::intrinsics::unreachable() },
                }
            } else {
                true
            }
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
            let _: ::std::cmp::AssertParamIsEq<xgrandpa::Log<Runtime>>;
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_ENCODE_FOR_InternalLog: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate parity_codec as _parity_codec;
    impl _parity_codec::Encode for InternalLog {
        fn encode_to<EncOut: _parity_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                InternalLog::system(ref aa) => {
                    dest.push_byte(0usize as u8);
                    dest.push(aa);
                }
                InternalLog::consensus(ref aa) => {
                    dest.push_byte(1usize as u8);
                    dest.push(aa);
                }
                InternalLog::xgrandpa(ref aa) => {
                    dest.push_byte(2usize as u8);
                    dest.push(aa);
                }
                _ => (),
            }
        }
    }
};
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DECODE_FOR_InternalLog: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate parity_codec as _parity_codec;
    impl _parity_codec::Decode for InternalLog {
        fn decode<DecIn: _parity_codec::Input>(input: &mut DecIn) -> Option<Self> {
            match input.read_byte()? {
                x if x == 0usize as u8 => {
                    Some(InternalLog::system(_parity_codec::Decode::decode(input)?))
                }
                x if x == 1usize as u8 => Some(InternalLog::consensus(
                    _parity_codec::Decode::decode(input)?,
                )),
                x if x == 2usize as u8 => {
                    Some(InternalLog::xgrandpa(_parity_codec::Decode::decode(input)?))
                }
                _ => None,
            }
        }
    }
};
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
            (&InternalLog::xgrandpa(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("xgrandpa");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_InternalLog: () = {
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
        fn serialize<__S>(&self, __serializer: __S) -> _serde::export::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            match *self {
                InternalLog::system(ref __field0) => _serde::Serializer::serialize_newtype_variant(
                    __serializer,
                    "InternalLog",
                    0u32,
                    "system",
                    __field0,
                ),
                InternalLog::consensus(ref __field0) => {
                    _serde::Serializer::serialize_newtype_variant(
                        __serializer,
                        "InternalLog",
                        1u32,
                        "consensus",
                        __field0,
                    )
                }
                InternalLog::xgrandpa(ref __field0) => {
                    _serde::Serializer::serialize_newtype_variant(
                        __serializer,
                        "InternalLog",
                        2u32,
                        "xgrandpa",
                        __field0,
                    )
                }
            }
        }
    }
};
impl Log {
    /// Try to convert `$name` into `generic::DigestItemRef`. Returns Some when
    /// `self` is a 'system' log && it has been marked as 'system' in macro call.
    /// Otherwise, None is returned.
    #[allow(unreachable_patterns)]
    fn dref<'a>(
        &'a self,
    ) -> Option<::sr_primitives::generic::DigestItemRef<'a, Hash, AuthorityId, AuthoritySignature>>
    {
        match self.0 {
            InternalLog::system(system::RawLog::ChangesTrieRoot(ref v)) => {
                Some(::sr_primitives::generic::DigestItemRef::ChangesTrieRoot(v))
            }
            InternalLog::consensus(consensus::RawLog::AuthoritiesChange(ref v)) => Some(
                ::sr_primitives::generic::DigestItemRef::AuthoritiesChange(v),
            ),
            _ => None,
        }
    }
}
impl ::sr_primitives::traits::DigestItem for Log {
    type
    Hash
    =
    <::sr_primitives::generic::DigestItem<Hash, AuthorityId,
                                          AuthoritySignature> as
    ::sr_primitives::traits::DigestItem>::Hash;
    type
    AuthorityId
    =
    <::sr_primitives::generic::DigestItem<Hash, AuthorityId,
                                          AuthoritySignature> as
    ::sr_primitives::traits::DigestItem>::AuthorityId;
    fn as_authorities_change(&self) -> Option<&[Self::AuthorityId]> {
        self.dref().and_then(|dref| dref.as_authorities_change())
    }
    fn as_changes_trie_root(&self) -> Option<&Self::Hash> {
        self.dref().and_then(|dref| dref.as_changes_trie_root())
    }
}
impl From<::sr_primitives::generic::DigestItem<Hash, AuthorityId, AuthoritySignature>> for Log {
    /// Converts `generic::DigestItem` into `$name`. If `generic::DigestItem` represents
    /// a system item which is supported by the runtime, it is returned.
    /// Otherwise we expect a `Other` log item. Trying to convert from anything other
    /// will lead to panic in runtime, since the runtime does not supports this 'system'
    /// log item.
    #[allow(unreachable_patterns)]
    fn from(
        gen: ::sr_primitives::generic::DigestItem<Hash, AuthorityId, AuthoritySignature>,
    ) -> Self {
        match gen {
            ::sr_primitives::generic::DigestItem::ChangesTrieRoot(value) => {
                Log(InternalLog::system(system::RawLog::ChangesTrieRoot(value)))
            }
            ::sr_primitives::generic::DigestItem::AuthoritiesChange(value) => Log(
                InternalLog::consensus(consensus::RawLog::AuthoritiesChange(value)),
            ),
            _ => gen
                .as_other()
                .and_then(|value| ::sr_primitives::codec::Decode::decode(&mut &value[..]))
                .map(Log)
                .expect("not allowed to fail in runtime"),
        }
    }
}
impl ::sr_primitives::codec::Decode for Log {
    /// `generic::DigestItem` binary compatible decode.
    fn decode<I: ::sr_primitives::codec::Input>(input: &mut I) -> Option<Self> {
        let gen: ::sr_primitives::generic::DigestItem<Hash, AuthorityId, AuthoritySignature> =
            ::sr_primitives::codec::Decode::decode(input)?;
        Some(Log::from(gen))
    }
}
impl ::sr_primitives::codec::Encode for Log {
    /// `generic::DigestItem` binary compatible encode.
    fn encode(&self) -> Vec<u8> {
        match self.dref() {
            Some(dref) => dref.encode(),
            None => {
                let gen: ::sr_primitives::generic::DigestItem<
                    Hash,
                    AuthorityId,
                    AuthoritySignature,
                > = ::sr_primitives::generic::DigestItem::Other(self.0.encode());
                gen.encode()
            }
        }
    }
}
impl From<system::Log<Runtime>> for Log {
    /// Converts single module log item into `$name`.
    fn from(x: system::Log<Runtime>) -> Self {
        Log(x.into())
    }
}
impl From<system::Log<Runtime>> for InternalLog {
    /// Converts single module log item into `$internal`.
    fn from(x: system::Log<Runtime>) -> Self {
        InternalLog::system(x)
    }
}
impl From<consensus::Log<Runtime>> for Log {
    /// Converts single module log item into `$name`.
    fn from(x: consensus::Log<Runtime>) -> Self {
        Log(x.into())
    }
}
impl From<consensus::Log<Runtime>> for InternalLog {
    /// Converts single module log item into `$internal`.
    fn from(x: consensus::Log<Runtime>) -> Self {
        InternalLog::consensus(x)
    }
}
impl From<xgrandpa::Log<Runtime>> for Log {
    /// Converts single module log item into `$name`.
    fn from(x: xgrandpa::Log<Runtime>) -> Self {
        Log(x.into())
    }
}
impl From<xgrandpa::Log<Runtime>> for InternalLog {
    /// Converts single module log item into `$internal`.
    fn from(x: xgrandpa::Log<Runtime>) -> Self {
        InternalLog::xgrandpa(x)
    }
}
#[cfg(any(feature = "std", test))]
pub type SystemConfig = system::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type TimestampConfig = timestamp::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type ConsensusConfig = consensus::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type SessionConfig = xsession::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XSystemConfig = xsystem::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XFeeManagerConfig = xfee_manager::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XAssetsConfig = xassets::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XAssetsProcessConfig = xprocess::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XStakingConfig = xstaking::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XTokensConfig = xtokens::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XSpotConfig = xspot::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XBridgeOfBTCConfig = xbitcoin::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XBridgeOfSDOTConfig = xsdot::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XBridgeFeaturesConfig = xbridge_features::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
pub type XBootstrapConfig = xbootstrap::GenesisConfig<Runtime>;
#[cfg(any(feature = "std", test))]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct GenesisConfig {
    pub system: Option<SystemConfig>,
    pub timestamp: Option<TimestampConfig>,
    pub consensus: Option<ConsensusConfig>,
    pub xsession: Option<SessionConfig>,
    pub xsystem: Option<XSystemConfig>,
    pub xfee_manager: Option<XFeeManagerConfig>,
    pub xassets: Option<XAssetsConfig>,
    pub xprocess: Option<XAssetsProcessConfig>,
    pub xstaking: Option<XStakingConfig>,
    pub xtokens: Option<XTokensConfig>,
    pub xspot: Option<XSpotConfig>,
    pub xbitcoin: Option<XBridgeOfBTCConfig>,
    pub xsdot: Option<XBridgeOfSDOTConfig>,
    pub xbridge_features: Option<XBridgeFeaturesConfig>,
    pub xbootstrap: Option<XBootstrapConfig>,
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_GenesisConfig: () = {
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
        fn serialize<__S>(&self, __serializer: __S) -> _serde::export::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = match _serde::Serializer::serialize_struct(
                __serializer,
                "GenesisConfig",
                false as usize + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "system",
                &self.system,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "timestamp",
                &self.timestamp,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "consensus",
                &self.consensus,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xsession",
                &self.xsession,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xsystem",
                &self.xsystem,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xfeeManager",
                &self.xfee_manager,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xassets",
                &self.xassets,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xprocess",
                &self.xprocess,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xstaking",
                &self.xstaking,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xtokens",
                &self.xtokens,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xspot",
                &self.xspot,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xbitcoin",
                &self.xbitcoin,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xsdot",
                &self.xsdot,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xbridgeFeatures",
                &self.xbridge_features,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "xbootstrap",
                &self.xbootstrap,
            ) {
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
const _IMPL_DESERIALIZE_FOR_GenesisConfig: () = {
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
    impl<'de> _serde::Deserialize<'de> for GenesisConfig {
        fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
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
                __field14,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
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
                        14u64 => _serde::export::Ok(__Field::__field14),
                        _ => _serde::export::Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(__value),
                            &"field index 0 <= i < 15",
                        )),
                    }
                }
                fn visit_str<__E>(self, __value: &str) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "system" => _serde::export::Ok(__Field::__field0),
                        "timestamp" => _serde::export::Ok(__Field::__field1),
                        "consensus" => _serde::export::Ok(__Field::__field2),
                        "xsession" => _serde::export::Ok(__Field::__field3),
                        "xsystem" => _serde::export::Ok(__Field::__field4),
                        "xfeeManager" => _serde::export::Ok(__Field::__field5),
                        "xassets" => _serde::export::Ok(__Field::__field6),
                        "xprocess" => _serde::export::Ok(__Field::__field7),
                        "xstaking" => _serde::export::Ok(__Field::__field8),
                        "xtokens" => _serde::export::Ok(__Field::__field9),
                        "xspot" => _serde::export::Ok(__Field::__field10),
                        "xbitcoin" => _serde::export::Ok(__Field::__field11),
                        "xsdot" => _serde::export::Ok(__Field::__field12),
                        "xbridgeFeatures" => _serde::export::Ok(__Field::__field13),
                        "xbootstrap" => _serde::export::Ok(__Field::__field14),
                        _ => _serde::export::Err(_serde::de::Error::unknown_field(__value, FIELDS)),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"system" => _serde::export::Ok(__Field::__field0),
                        b"timestamp" => _serde::export::Ok(__Field::__field1),
                        b"consensus" => _serde::export::Ok(__Field::__field2),
                        b"xsession" => _serde::export::Ok(__Field::__field3),
                        b"xsystem" => _serde::export::Ok(__Field::__field4),
                        b"xfeeManager" => _serde::export::Ok(__Field::__field5),
                        b"xassets" => _serde::export::Ok(__Field::__field6),
                        b"xprocess" => _serde::export::Ok(__Field::__field7),
                        b"xstaking" => _serde::export::Ok(__Field::__field8),
                        b"xtokens" => _serde::export::Ok(__Field::__field9),
                        b"xspot" => _serde::export::Ok(__Field::__field10),
                        b"xbitcoin" => _serde::export::Ok(__Field::__field11),
                        b"xsdot" => _serde::export::Ok(__Field::__field12),
                        b"xbridgeFeatures" => _serde::export::Ok(__Field::__field13),
                        b"xbootstrap" => _serde::export::Ok(__Field::__field14),
                        _ => {
                            let __value = &_serde::export::from_utf8_lossy(__value);
                            _serde::export::Err(_serde::de::Error::unknown_field(__value, FIELDS))
                        }
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor<'de> {
                marker: _serde::export::PhantomData<GenesisConfig>,
                lifetime: _serde::export::PhantomData<&'de ()>,
            }
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = GenesisConfig;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "struct GenesisConfig")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::export::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match match _serde::de::SeqAccess::next_element::<
                        Option<SystemConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                0usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field1 = match match _serde::de::SeqAccess::next_element::<
                        Option<TimestampConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                1usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field2 = match match _serde::de::SeqAccess::next_element::<
                        Option<ConsensusConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                2usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field3 = match match _serde::de::SeqAccess::next_element::<
                        Option<SessionConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                3usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field4 = match match _serde::de::SeqAccess::next_element::<
                        Option<XSystemConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                4usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field5 = match match _serde::de::SeqAccess::next_element::<
                        Option<XFeeManagerConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                5usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field6 = match match _serde::de::SeqAccess::next_element::<
                        Option<XAssetsConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                6usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field7 = match match _serde::de::SeqAccess::next_element::<
                        Option<XAssetsProcessConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                7usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field8 = match match _serde::de::SeqAccess::next_element::<
                        Option<XStakingConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                8usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field9 = match match _serde::de::SeqAccess::next_element::<
                        Option<XTokensConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                9usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field10 = match match _serde::de::SeqAccess::next_element::<
                        Option<XSpotConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                10usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field11 = match match _serde::de::SeqAccess::next_element::<
                        Option<XBridgeOfBTCConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                11usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field12 = match match _serde::de::SeqAccess::next_element::<
                        Option<XBridgeOfSDOTConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                12usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field13 = match match _serde::de::SeqAccess::next_element::<
                        Option<XBridgeFeaturesConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                13usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    let __field14 = match match _serde::de::SeqAccess::next_element::<
                        Option<XBootstrapConfig>,
                    >(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                14usize,
                                &"struct GenesisConfig with 15 elements",
                            ));
                        }
                    };
                    _serde::export::Ok(GenesisConfig {
                        system: __field0,
                        timestamp: __field1,
                        consensus: __field2,
                        xsession: __field3,
                        xsystem: __field4,
                        xfee_manager: __field5,
                        xassets: __field6,
                        xprocess: __field7,
                        xstaking: __field8,
                        xtokens: __field9,
                        xspot: __field10,
                        xbitcoin: __field11,
                        xsdot: __field12,
                        xbridge_features: __field13,
                        xbootstrap: __field14,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::export::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::export::Option<Option<SystemConfig>> =
                        _serde::export::None;
                    let mut __field1: _serde::export::Option<Option<TimestampConfig>> =
                        _serde::export::None;
                    let mut __field2: _serde::export::Option<Option<ConsensusConfig>> =
                        _serde::export::None;
                    let mut __field3: _serde::export::Option<Option<SessionConfig>> =
                        _serde::export::None;
                    let mut __field4: _serde::export::Option<Option<XSystemConfig>> =
                        _serde::export::None;
                    let mut __field5: _serde::export::Option<Option<XFeeManagerConfig>> =
                        _serde::export::None;
                    let mut __field6: _serde::export::Option<Option<XAssetsConfig>> =
                        _serde::export::None;
                    let mut __field7: _serde::export::Option<Option<XAssetsProcessConfig>> =
                        _serde::export::None;
                    let mut __field8: _serde::export::Option<Option<XStakingConfig>> =
                        _serde::export::None;
                    let mut __field9: _serde::export::Option<Option<XTokensConfig>> =
                        _serde::export::None;
                    let mut __field10: _serde::export::Option<Option<XSpotConfig>> =
                        _serde::export::None;
                    let mut __field11: _serde::export::Option<Option<XBridgeOfBTCConfig>> =
                        _serde::export::None;
                    let mut __field12: _serde::export::Option<Option<XBridgeOfSDOTConfig>> =
                        _serde::export::None;
                    let mut __field13: _serde::export::Option<Option<XBridgeFeaturesConfig>> =
                        _serde::export::None;
                    let mut __field14: _serde::export::Option<Option<XBootstrapConfig>> =
                        _serde::export::None;
                    while let _serde::export::Some(__key) =
                        match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        }
                    {
                        match __key {
                            __Field::__field0 => {
                                if _serde::export::Option::is_some(&__field0) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "system",
                                        ),
                                    );
                                }
                                __field0 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<SystemConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field1 => {
                                if _serde::export::Option::is_some(&__field1) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "timestamp",
                                        ),
                                    );
                                }
                                __field1 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<TimestampConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field2 => {
                                if _serde::export::Option::is_some(&__field2) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "consensus",
                                        ),
                                    );
                                }
                                __field2 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<ConsensusConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field3 => {
                                if _serde::export::Option::is_some(&__field3) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xsession",
                                        ),
                                    );
                                }
                                __field3 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<SessionConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field4 => {
                                if _serde::export::Option::is_some(&__field4) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xsystem",
                                        ),
                                    );
                                }
                                __field4 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<XSystemConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field5 => {
                                if _serde::export::Option::is_some(&__field5) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xfeeManager",
                                        ),
                                    );
                                }
                                __field5 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        Option<XFeeManagerConfig>,
                                    >(&mut __map)
                                    {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field6 => {
                                if _serde::export::Option::is_some(&__field6) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xassets",
                                        ),
                                    );
                                }
                                __field6 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<XAssetsConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field7 => {
                                if _serde::export::Option::is_some(&__field7) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xprocess",
                                        ),
                                    );
                                }
                                __field7 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        Option<XAssetsProcessConfig>,
                                    >(&mut __map)
                                    {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field8 => {
                                if _serde::export::Option::is_some(&__field8) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xstaking",
                                        ),
                                    );
                                }
                                __field8 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<XStakingConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field9 => {
                                if _serde::export::Option::is_some(&__field9) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xtokens",
                                        ),
                                    );
                                }
                                __field9 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<XTokensConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field10 => {
                                if _serde::export::Option::is_some(&__field10) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("xspot"),
                                    );
                                }
                                __field10 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<Option<XSpotConfig>>(
                                        &mut __map,
                                    ) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field11 => {
                                if _serde::export::Option::is_some(&__field11) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xbitcoin",
                                        ),
                                    );
                                }
                                __field11 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        Option<XBridgeOfBTCConfig>,
                                    >(&mut __map)
                                    {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field12 => {
                                if _serde::export::Option::is_some(&__field12) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("xsdot"),
                                    );
                                }
                                __field12 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        Option<XBridgeOfSDOTConfig>,
                                    >(&mut __map)
                                    {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field13 => {
                                if _serde::export::Option::is_some(&__field13) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xbridgeFeatures",
                                        ),
                                    );
                                }
                                __field13 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        Option<XBridgeFeaturesConfig>,
                                    >(&mut __map)
                                    {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field14 => {
                                if _serde::export::Option::is_some(&__field14) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "xbootstrap",
                                        ),
                                    );
                                }
                                __field14 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        Option<XBootstrapConfig>,
                                    >(&mut __map)
                                    {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::export::Some(__field0) => __field0,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("system") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::export::Some(__field1) => __field1,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("timestamp") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::export::Some(__field2) => __field2,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("consensus") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field3 = match __field3 {
                        _serde::export::Some(__field3) => __field3,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("xsession") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field4 = match __field4 {
                        _serde::export::Some(__field4) => __field4,
                        _serde::export::None => match _serde::private::de::missing_field("xsystem")
                        {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        },
                    };
                    let __field5 = match __field5 {
                        _serde::export::Some(__field5) => __field5,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("xfeeManager") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field6 = match __field6 {
                        _serde::export::Some(__field6) => __field6,
                        _serde::export::None => match _serde::private::de::missing_field("xassets")
                        {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        },
                    };
                    let __field7 = match __field7 {
                        _serde::export::Some(__field7) => __field7,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("xprocess") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field8 = match __field8 {
                        _serde::export::Some(__field8) => __field8,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("xstaking") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field9 = match __field9 {
                        _serde::export::Some(__field9) => __field9,
                        _serde::export::None => match _serde::private::de::missing_field("xtokens")
                        {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        },
                    };
                    let __field10 = match __field10 {
                        _serde::export::Some(__field10) => __field10,
                        _serde::export::None => match _serde::private::de::missing_field("xspot") {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        },
                    };
                    let __field11 = match __field11 {
                        _serde::export::Some(__field11) => __field11,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("xbitcoin") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field12 = match __field12 {
                        _serde::export::Some(__field12) => __field12,
                        _serde::export::None => match _serde::private::de::missing_field("xsdot") {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        },
                    };
                    let __field13 = match __field13 {
                        _serde::export::Some(__field13) => __field13,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("xbridgeFeatures") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field14 = match __field14 {
                        _serde::export::Some(__field14) => __field14,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("xbootstrap") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    _serde::export::Ok(GenesisConfig {
                        system: __field0,
                        timestamp: __field1,
                        consensus: __field2,
                        xsession: __field3,
                        xsystem: __field4,
                        xfee_manager: __field5,
                        xassets: __field6,
                        xprocess: __field7,
                        xstaking: __field8,
                        xtokens: __field9,
                        xspot: __field10,
                        xbitcoin: __field11,
                        xsdot: __field12,
                        xbridge_features: __field13,
                        xbootstrap: __field14,
                    })
                }
            }
            const FIELDS: &'static [&'static str] = &[
                "system",
                "timestamp",
                "consensus",
                "xsession",
                "xsystem",
                "xfeeManager",
                "xassets",
                "xprocess",
                "xstaking",
                "xtokens",
                "xspot",
                "xbitcoin",
                "xsdot",
                "xbridgeFeatures",
                "xbootstrap",
            ];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "GenesisConfig",
                FIELDS,
                __Visitor {
                    marker: _serde::export::PhantomData::<GenesisConfig>,
                    lifetime: _serde::export::PhantomData,
                },
            )
        }
    }
};
#[cfg(any(feature = "std", test))]
impl ::sr_primitives::BuildStorage for GenesisConfig {
    fn assimilate_storage(
        self,
        top: &mut ::sr_primitives::StorageOverlay,
        children: &mut ::sr_primitives::ChildrenStorageOverlay,
    ) -> ::std::result::Result<(), String> {
        if let Some(extra) = self.system {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.timestamp {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.consensus {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xsession {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xsystem {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xfee_manager {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xassets {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xprocess {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xstaking {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xtokens {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xspot {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xbitcoin {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xsdot {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xbridge_features {
            extra.assimilate_storage(top, children)?;
        }
        if let Some(extra) = self.xbootstrap {
            extra.assimilate_storage(top, children)?;
        }
        Ok(())
    }
}
trait InherentDataExt {
    fn create_extrinsics(
        &self,
    ) -> ::srml_support::inherent::Vec<<Block as ::srml_support::inherent::BlockT>::Extrinsic>;
    fn check_extrinsics(&self, block: &Block) -> ::srml_support::inherent::CheckInherentsResult;
}
impl InherentDataExt for ::srml_support::inherent::InherentData {
    fn create_extrinsics(
        &self,
    ) -> ::srml_support::inherent::Vec<<Block as ::srml_support::inherent::BlockT>::Extrinsic> {
        use srml_support::inherent::ProvideInherent;
        let mut inherents = Vec::new();
        if let Some(inherent) = Timestamp::create_inherent(self) {
            inherents.push(UncheckedExtrinsic::new_unsigned(Call::Timestamp(inherent)));
        }
        if let Some(inherent) = Consensus::create_inherent(self) {
            inherents.push(UncheckedExtrinsic::new_unsigned(Call::Consensus(inherent)));
        }
        if let Some(inherent) = FinalityTracker::create_inherent(self) {
            inherents.push(UncheckedExtrinsic::new_unsigned(Call::FinalityTracker(
                inherent,
            )));
        }
        if let Some(inherent) = Aura::create_inherent(self) {
            inherents.push(UncheckedExtrinsic::new_unsigned(Call::Timestamp(inherent)));
        }
        if let Some(inherent) = XSystem::create_inherent(self) {
            inherents.push(UncheckedExtrinsic::new_unsigned(Call::XSystem(inherent)));
        }
        inherents
    }
    fn check_extrinsics(&self, block: &Block) -> ::srml_support::inherent::CheckInherentsResult {
        use srml_support::inherent::{IsFatalError, ProvideInherent};
        let mut result = ::srml_support::inherent::CheckInherentsResult::new();
        for xt in block.extrinsics() {
            if ::srml_support::inherent::Extrinsic::is_signed(xt).unwrap_or(false) {
                break;
            }
            match xt.function {
                Call::Timestamp(ref call) => {
                    if let Err(e) = Timestamp::check_inherent(call, self) {
                        result
                            .put_error(Timestamp::INHERENT_IDENTIFIER, &e)
                            .expect("There is only one fatal error; qed");
                        if e.is_fatal_error() {
                            return result;
                        }
                    }
                }
                _ => {}
            }
            match xt.function {
                Call::Consensus(ref call) => {
                    if let Err(e) = Consensus::check_inherent(call, self) {
                        result
                            .put_error(Consensus::INHERENT_IDENTIFIER, &e)
                            .expect("There is only one fatal error; qed");
                        if e.is_fatal_error() {
                            return result;
                        }
                    }
                }
                _ => {}
            }
            match xt.function {
                Call::FinalityTracker(ref call) => {
                    if let Err(e) = FinalityTracker::check_inherent(call, self) {
                        result
                            .put_error(FinalityTracker::INHERENT_IDENTIFIER, &e)
                            .expect("There is only one fatal error; qed");
                        if e.is_fatal_error() {
                            return result;
                        }
                    }
                }
                _ => {}
            }
            match xt.function {
                Call::Timestamp(ref call) => {
                    if let Err(e) = Aura::check_inherent(call, self) {
                        result
                            .put_error(Aura::INHERENT_IDENTIFIER, &e)
                            .expect("There is only one fatal error; qed");
                        if e.is_fatal_error() {
                            return result;
                        }
                    }
                }
                _ => {}
            }
            match xt.function {
                Call::XSystem(ref call) => {
                    if let Err(e) = XSystem::check_inherent(call, self) {
                        result
                            .put_error(XSystem::INHERENT_IDENTIFIER, &e)
                            .expect("There is only one fatal error; qed");
                        if e.is_fatal_error() {
                            return result;
                        }
                    }
                }
                _ => {}
            }
        }
        result
    }
}
impl ::srml_support::unsigned::ValidateUnsigned for Runtime {
    type Call = Call;
    fn validate_unsigned(call: &Self::Call) -> ::srml_support::unsigned::TransactionValidity {
        #[allow(unreachable_patterns)]
        match call {
            _ => ::srml_support::unsigned::TransactionValidity::Invalid(
                ::srml_support::unsigned::ApplyError::BadSignature as i8,
            ),
        }
    }
}
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
#[doc(hidden)]
mod sr_api_hidden_includes_IMPL_RUNTIME_APIS {
    pub extern crate client as sr_api_client;
}
pub struct RuntimeApi {}
#[doc = r" Implements all runtime apis for the client side."]
#[cfg(any(feature = "std", test))]
pub struct RuntimeApiImpl<C: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                       as
                                                                                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
                          'static> {
    call: &'static C,
    commit_on_success: std::cell::RefCell<bool>,
    initialized_block: std::cell::RefCell<Option<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                     as
                                                                                                                                     self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>>,
    changes: std::cell::RefCell<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::OverlayedChanges>,
    recorder: Option<std::rc::Rc<std::cell::RefCell<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ProofRecorder<<Runtime
                                                                                                                                              as
                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>>>,
}
#[cfg(any(feature = "std", test))]
unsafe impl <C: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                          as
                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>
 Send for RuntimeApiImpl<C> {
}
#[cfg(any(feature = "std", test))]
unsafe impl <C: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                          as
                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>
 Sync for RuntimeApiImpl<C> {
}
#[cfg(any(feature = "std", test))]
impl <C: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                   as
                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>
 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ApiExt<<Runtime
                                                                                    as
                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<C> {
    fn map_api_result<F: FnOnce(&Self) -> ::std::result::Result<R, E>, R,
                      E>(&self, map_call: F) -> ::std::result::Result<R, E>
     where Self: Sized {
        *self.commit_on_success.borrow_mut() = false;
        let res = map_call(self);
        *self.commit_on_success.borrow_mut() = true;
        self.commit_on_ok(&res);
        res
    }
    fn runtime_version_at(&self,
                          at:
                              &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                   as
                                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::RuntimeVersion> {
        self.call.runtime_version_at(at)
    }
    fn record_proof(&mut self) { self.recorder = Some(Default::default()); }
    fn extract_proof(&mut self) -> Option<Vec<Vec<u8>>> {
        self.recorder.take().map(|r|
                                     {
                                         r.borrow_mut().drain().into_iter().map(|n|
                                                                                    n.data.to_vec()).collect()
                                     })
    }
}
#[cfg(any(feature = "std", test))]
impl <C: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                   as
                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ConstructRuntimeApi<<Runtime
                                                                                                 as
                                                                                                 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                 C>
 for RuntimeApi {
    type
    RuntimeApi
    =
    RuntimeApiImpl<C>;
    fn construct_runtime_api<'a>(call: &'a C)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ApiRef<'a,
                                                                                            Self::RuntimeApi> {
        RuntimeApiImpl{call: unsafe { ::std::mem::transmute(call) },
                       commit_on_success: true.into(),
                       initialized_block: None.into(),
                       changes: Default::default(),
                       recorder: Default::default(),}.into()
    }
}
#[cfg(any(feature = "std", test))]
impl <C: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                   as
                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>
 RuntimeApiImpl<C> {
    fn call_api_at<R: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode +
                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode +
                   PartialEq,
                   F: FnOnce(&C, &Self,
                             &std::cell::RefCell<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::OverlayedChanges>,
                             &std::cell::RefCell<Option<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                            as
                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>>,
                             &Option<std::rc::Rc<std::cell::RefCell<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ProofRecorder<<Runtime
                                                                                                                                                              as
                                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>>>)
                   ->
                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<R>>>(&self,
                                                                                                                                                                                                 call_api_at:
                                                                                                                                                                                                     F)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<R>> {
        let res =
            unsafe {
                call_api_at(&self.call, self, &self.changes,
                            &self.initialized_block, &self.recorder)
            };
        self.commit_on_ok(&res);
        res
    }
    fn commit_on_ok<R, E>(&self, res: &::std::result::Result<R, E>) {
        if *self.commit_on_success.borrow() {
            if res.is_err() {
                self.changes.borrow_mut().discard_prospective();
            } else { self.changes.borrow_mut().commit_prospective(); }
        }
    }
}
impl client_api::runtime_decl_for_Core::Core<Block> for Runtime {
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
        {
            ::std::rt::begin_panic(
                "Deprecated, please use `AuthoritiesApi`.",
                &("runtime/src/lib.rs", 284u32, 13u32),
            )
        }
    }
}
impl client_api::runtime_decl_for_Metadata::Metadata<Block> for Runtime {
    fn metadata() -> OpaqueMetadata {
        Runtime::metadata().into()
    }
}
impl block_builder_api::runtime_decl_for_BlockBuilder::BlockBuilder<Block> for Runtime {
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
impl client_api::runtime_decl_for_TaggedTransactionQueue::TaggedTransactionQueue<Block>
    for Runtime
{
    fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
        Executive::validate_transaction(tx)
    }
}
impl offchain_primitives::runtime_decl_for_OffchainWorkerApi::OffchainWorkerApi<Block> for Runtime {
    fn offchain_worker(number: NumberFor<Block>) {
        Executive::offchain_worker(number)
    }
}
impl fg_primitives::runtime_decl_for_GrandpaApi::GrandpaApi<Block> for Runtime {
    fn grandpa_pending_change(
        digest: &DigestFor<Block>,
    ) -> Option<ScheduledChange<NumberFor<Block>>> {
        for log in digest.logs.iter().filter_map(|l| match l {
            Log(InternalLog::xgrandpa(grandpa_signal)) => Some(grandpa_signal),
            _ => None,
        }) {
            if let Some(change) = Grandpa::scrape_digest_change(log) {
                return Some(change);
            }
        }
        None
    }
    fn grandpa_forced_change(
        digest: &DigestFor<Block>,
    ) -> Option<(NumberFor<Block>, ScheduledChange<NumberFor<Block>>)> {
        for log in digest.logs.iter().filter_map(|l| match l {
            Log(InternalLog::xgrandpa(grandpa_signal)) => Some(grandpa_signal),
            _ => None,
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
impl consensus_aura::runtime_decl_for_AuraApi::AuraApi<Block> for Runtime {
    fn slot_duration() -> u64 {
        Aura::slot_duration()
    }
}
impl consensus_authorities::runtime_decl_for_AuthoritiesApi::AuthoritiesApi<Block> for Runtime {
    fn authorities() -> Vec<AuthorityIdFor<Block>> {
        Consensus::authorities()
    }
}
impl runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block> for Runtime {
    fn valid_assets() -> Vec<xassets::Token> {
        XAssets::valid_assets()
    }
    fn all_assets() -> Vec<(xassets::Asset, bool)> {
        XAssets::all_assets()
    }
    fn valid_assets_of(
        who: AccountId,
    ) -> Vec<(xassets::Token, BTreeMap<xassets::AssetType, Balance>)> {
        XAssets::valid_assets_of(&who)
    }
    fn withdrawal_list_of(
        chain: xassets::Chain,
    ) -> Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, TimestampU64>> {
        match chain {
            xassets::Chain::Bitcoin => XBridgeOfBTC::withdrawal_list(),
            xassets::Chain::Ethereum => Vec::new(),
            _ => Vec::new(),
        }
    }
    fn deposit_list_of(
        chain: xassets::Chain,
    ) -> Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, TimestampU64>> {
        match chain {
            xassets::Chain::Bitcoin => XBridgeOfBTC::deposit_list(),
            xassets::Chain::Ethereum => Vec::new(),
            _ => Vec::new(),
        }
    }
    fn verify_address(
        token: xassets::Token,
        addr: AddrStr,
        ext: xassets::Memo,
    ) -> Result<(), Vec<u8>> {
        XAssetsProcess::verify_address(token, addr, ext).map_err(|e| e.as_bytes().to_vec())
    }
    fn withdrawal_limit(token: xassets::Token) -> Option<xprocess::WithdrawalLimit<Balance>> {
        XAssetsProcess::withdrawal_limit(&token)
    }
}
impl runtime_api::xmining_api::runtime_decl_for_XMiningApi::XMiningApi<Block> for Runtime {
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
impl runtime_api::xspot_api::runtime_decl_for_XSpotApi::XSpotApi<Block> for Runtime {
    fn aver_asset_price(token: xassets::Token) -> Option<Balance> {
        XSpot::aver_asset_price(&token)
    }
}
impl runtime_api::xfee_api::runtime_decl_for_XFeeApi::XFeeApi<Block> for Runtime {
    fn transaction_fee(call_params: Vec<u8>, encoded_len: u64) -> Option<u64> {
        use fee::CheckFee;
        let call: Call = if let Some(call) = Decode::decode(&mut call_params.as_slice()) {
            call
        } else {
            return None;
        };
        let switch = xfee_manager::SwitchStore::default();
        call.check_fee(switch)
            .map(|power| XFeeManager::transaction_fee(power, encoded_len))
    }
}
impl runtime_api::xsession_api::runtime_decl_for_XSessionApi::XSessionApi<Block> for Runtime {
    fn pubkeys_for_validator_name(name: Vec<u8>) -> Option<(AccountId, Option<AuthorityId>)> {
        Session::pubkeys_for_validator_name(name)
    }
}
impl runtime_api::xstaking_api::runtime_decl_for_XStakingApi::XStakingApi<Block> for Runtime {
    fn intention_set() -> Vec<AccountId> {
        XStaking::intention_set()
    }
}
impl runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::XBridgeApi<Block> for Runtime {
    fn mock_new_trustees(
        chain: xassets::Chain,
        candidates: Vec<AccountId>,
    ) -> Result<GenericAllSessionInfo<AccountId>, Vec<u8>> {
        XBridgeFeatures::mock_trustee_session_impl(chain, candidates)
            .map_err(|e| e.as_bytes().to_vec())
    }
    fn trustee_props_for(who: AccountId) -> BTreeMap<xassets::Chain, GenericTrusteeIntentionProps> {
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
    fn trustee_session_info_for(
        chain: xassets::Chain,
    ) -> Option<(u32, GenericAllSessionInfo<AccountId>)> {
        XBridgeFeatures::current_trustee_session_info_for(chain)
            .map(|info| ((XBridgeFeatures::current_session_number(chain), info)))
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 client_api::Core<<Runtime as
                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn Core_version_runtime_api_impl(&self,
                                     at:
                                         &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                              as
                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                     context:
                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                     params: Option<()>,
                                     params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<RuntimeVersion>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 client_api::runtime_decl_for_Core::version_call_api_at(call_runtime_at,
                                                                                        core_api,
                                                                                        at,
                                                                                        params_encoded,
                                                                                        changes,
                                                                                        initialized_block,
                                                                                        params.map(|p|
                                                                                                       {
                                                                                                           client_api::runtime_decl_for_Core::version_native_call_generator::<Runtime,
                                                                                                                                                                              <Runtime
                                                                                                                                                                              as
                                                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                              Block>()
                                                                                                       }),
                                                                                        context,
                                                                                        recorder)
                             })
    }
    fn Core_execute_block_runtime_api_impl(&self,
                                           at:
                                               &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                    as
                                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                           context:
                                               self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                           params:
                                               Option<(<Runtime as
                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock)>,
                                           params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<()>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 client_api::runtime_decl_for_Core::execute_block_call_api_at(call_runtime_at,
                                                                                              core_api,
                                                                                              at,
                                                                                              params_encoded,
                                                                                              changes,
                                                                                              initialized_block,
                                                                                              params.map(|p|
                                                                                                             {
                                                                                                                 client_api::runtime_decl_for_Core::execute_block_native_call_generator::<Runtime,
                                                                                                                                                                                          <Runtime
                                                                                                                                                                                          as
                                                                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                          Block>(p)
                                                                                                             }),
                                                                                              context,
                                                                                              recorder)
                             })
    }
    fn Core_initialize_block_runtime_api_impl(&self,
                                              at:
                                                  &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                       as
                                                                                                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                              context:
                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                              params:
                                                  Option<(&<<Runtime as
                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock
                                                           as
                                                           BlockT>::Header)>,
                                              params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<()>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 client_api::runtime_decl_for_Core::initialize_block_call_api_at(call_runtime_at,
                                                                                                 core_api,
                                                                                                 at,
                                                                                                 params_encoded,
                                                                                                 changes,
                                                                                                 initialized_block,
                                                                                                 params.map(|p|
                                                                                                                {
                                                                                                                    client_api::runtime_decl_for_Core::initialize_block_native_call_generator::<Runtime,
                                                                                                                                                                                                <Runtime
                                                                                                                                                                                                as
                                                                                                                                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                Block>(p)
                                                                                                                }),
                                                                                                 context,
                                                                                                 recorder)
                             })
    }
    fn Core_authorities_runtime_api_impl(&self,
                                         at:
                                             &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                  as
                                                                                                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                         context:
                                             self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                         params: Option<()>,
                                         params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<AuthorityId>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 client_api::runtime_decl_for_Core::authorities_call_api_at(call_runtime_at,
                                                                                            core_api,
                                                                                            at,
                                                                                            params_encoded,
                                                                                            changes,
                                                                                            initialized_block,
                                                                                            params.map(|p|
                                                                                                           {
                                                                                                               client_api::runtime_decl_for_Core::authorities_native_call_generator::<Runtime,
                                                                                                                                                                                      <Runtime
                                                                                                                                                                                      as
                                                                                                                                                                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                      Block>()
                                                                                                           }),
                                                                                            context,
                                                                                            recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 client_api::Metadata<<Runtime as
                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn Metadata_metadata_runtime_api_impl(&self,
                                          at:
                                              &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                   as
                                                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                          context:
                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                          params: Option<()>,
                                          params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<OpaqueMetadata>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 client_api::runtime_decl_for_Metadata::metadata_call_api_at(call_runtime_at,
                                                                                             core_api,
                                                                                             at,
                                                                                             params_encoded,
                                                                                             changes,
                                                                                             initialized_block,
                                                                                             params.map(|p|
                                                                                                            {
                                                                                                                client_api::runtime_decl_for_Metadata::metadata_native_call_generator::<Runtime,
                                                                                                                                                                                        <Runtime
                                                                                                                                                                                        as
                                                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                        Block>()
                                                                                                            }),
                                                                                             context,
                                                                                             recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 block_builder_api::BlockBuilder<<Runtime as
                                 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn BlockBuilder_apply_extrinsic_runtime_api_impl(&self,
                                                     at:
                                                         &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                              as
                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                     context:
                                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                     params:
                                                         Option<(<<Runtime as
                                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock
                                                                 as
                                                                 BlockT>::Extrinsic)>,
                                                     params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<ApplyResult>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 block_builder_api::runtime_decl_for_BlockBuilder::apply_extrinsic_call_api_at(call_runtime_at,
                                                                                                               core_api,
                                                                                                               at,
                                                                                                               params_encoded,
                                                                                                               changes,
                                                                                                               initialized_block,
                                                                                                               params.map(|p|
                                                                                                                              {
                                                                                                                                  block_builder_api::runtime_decl_for_BlockBuilder::apply_extrinsic_native_call_generator::<Runtime,
                                                                                                                                                                                                                            <Runtime
                                                                                                                                                                                                                            as
                                                                                                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                            Block>(p)
                                                                                                                              }),
                                                                                                               context,
                                                                                                               recorder)
                             })
    }
    fn BlockBuilder_finalize_block_runtime_api_impl(&self,
                                                    at:
                                                        &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                             as
                                                                                                                                             self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                    context:
                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                    params: Option<()>,
                                                    params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<<<Runtime
                                                                                                                                                                                   as
                                                                                                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock
                                                                                                                                                                                  as
                                                                                                                                                                                  BlockT>::Header>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 block_builder_api::runtime_decl_for_BlockBuilder::finalize_block_call_api_at(call_runtime_at,
                                                                                                              core_api,
                                                                                                              at,
                                                                                                              params_encoded,
                                                                                                              changes,
                                                                                                              initialized_block,
                                                                                                              params.map(|p|
                                                                                                                             {
                                                                                                                                 block_builder_api::runtime_decl_for_BlockBuilder::finalize_block_native_call_generator::<Runtime,
                                                                                                                                                                                                                          <Runtime
                                                                                                                                                                                                                          as
                                                                                                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                          Block>()
                                                                                                                             }),
                                                                                                              context,
                                                                                                              recorder)
                             })
    }
    fn BlockBuilder_inherent_extrinsics_runtime_api_impl(&self,
                                                         at:
                                                             &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                  as
                                                                                                                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                         context:
                                                             self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                         params:
                                                             Option<(InherentData)>,
                                                         params_encoded:
                                                             Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<<<Runtime
                                                                                                                                                                                       as
                                                                                                                                                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock
                                                                                                                                                                                      as
                                                                                                                                                                                      BlockT>::Extrinsic>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 block_builder_api::runtime_decl_for_BlockBuilder::inherent_extrinsics_call_api_at(call_runtime_at,
                                                                                                                   core_api,
                                                                                                                   at,
                                                                                                                   params_encoded,
                                                                                                                   changes,
                                                                                                                   initialized_block,
                                                                                                                   params.map(|p|
                                                                                                                                  {
                                                                                                                                      block_builder_api::runtime_decl_for_BlockBuilder::inherent_extrinsics_native_call_generator::<Runtime,
                                                                                                                                                                                                                                    <Runtime
                                                                                                                                                                                                                                    as
                                                                                                                                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                    Block>(p)
                                                                                                                                  }),
                                                                                                                   context,
                                                                                                                   recorder)
                             })
    }
    fn BlockBuilder_check_inherents_runtime_api_impl(&self,
                                                     at:
                                                         &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                              as
                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                     context:
                                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                     params:
                                                         Option<(<Runtime as
                                                                 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                 InherentData)>,
                                                     params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<CheckInherentsResult>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 block_builder_api::runtime_decl_for_BlockBuilder::check_inherents_call_api_at(call_runtime_at,
                                                                                                               core_api,
                                                                                                               at,
                                                                                                               params_encoded,
                                                                                                               changes,
                                                                                                               initialized_block,
                                                                                                               params.map(|p|
                                                                                                                              {
                                                                                                                                  block_builder_api::runtime_decl_for_BlockBuilder::check_inherents_native_call_generator::<Runtime,
                                                                                                                                                                                                                            <Runtime
                                                                                                                                                                                                                            as
                                                                                                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                            Block>(p.0,
                                                                                                                                                                                                                                   p.1)
                                                                                                                              }),
                                                                                                               context,
                                                                                                               recorder)
                             })
    }
    fn BlockBuilder_random_seed_runtime_api_impl(&self,
                                                 at:
                                                     &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                          as
                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                 context:
                                                     self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                 params: Option<()>,
                                                 params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<<<Runtime
                                                                                                                                                                                   as
                                                                                                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock
                                                                                                                                                                                  as
                                                                                                                                                                                  BlockT>::Hash>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 block_builder_api::runtime_decl_for_BlockBuilder::random_seed_call_api_at(call_runtime_at,
                                                                                                           core_api,
                                                                                                           at,
                                                                                                           params_encoded,
                                                                                                           changes,
                                                                                                           initialized_block,
                                                                                                           params.map(|p|
                                                                                                                          {
                                                                                                                              block_builder_api::runtime_decl_for_BlockBuilder::random_seed_native_call_generator::<Runtime,
                                                                                                                                                                                                                    <Runtime
                                                                                                                                                                                                                    as
                                                                                                                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                    Block>()
                                                                                                                          }),
                                                                                                           context,
                                                                                                           recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 client_api::TaggedTransactionQueue<<Runtime as
                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn TaggedTransactionQueue_validate_transaction_runtime_api_impl(&self,
                                                                    at:
                                                                        &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                             as
                                                                                                                                                             self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                                    context:
                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                                    params:
                                                                        Option<(<<Runtime
                                                                                 as
                                                                                 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock
                                                                                as
                                                                                BlockT>::Extrinsic)>,
                                                                    params_encoded:
                                                                        Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<TransactionValidity>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 client_api::runtime_decl_for_TaggedTransactionQueue::validate_transaction_call_api_at(call_runtime_at,
                                                                                                                       core_api,
                                                                                                                       at,
                                                                                                                       params_encoded,
                                                                                                                       changes,
                                                                                                                       initialized_block,
                                                                                                                       params.map(|p|
                                                                                                                                      {
                                                                                                                                          client_api::runtime_decl_for_TaggedTransactionQueue::validate_transaction_native_call_generator::<Runtime,
                                                                                                                                                                                                                                            <Runtime
                                                                                                                                                                                                                                            as
                                                                                                                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                            Block>(p)
                                                                                                                                      }),
                                                                                                                       context,
                                                                                                                       recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 offchain_primitives::OffchainWorkerApi<<Runtime as
                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn OffchainWorkerApi_offchain_worker_runtime_api_impl(&self,
                                                          at:
                                                              &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                   as
                                                                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                          context:
                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                          params:
                                                              Option<(NumberFor<<Runtime
                                                                                as
                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>)>,
                                                          params_encoded:
                                                              Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<()>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 offchain_primitives::runtime_decl_for_OffchainWorkerApi::offchain_worker_call_api_at(call_runtime_at,
                                                                                                                      core_api,
                                                                                                                      at,
                                                                                                                      params_encoded,
                                                                                                                      changes,
                                                                                                                      initialized_block,
                                                                                                                      params.map(|p|
                                                                                                                                     {
                                                                                                                                         offchain_primitives::runtime_decl_for_OffchainWorkerApi::offchain_worker_native_call_generator::<Runtime,
                                                                                                                                                                                                                                          <Runtime
                                                                                                                                                                                                                                          as
                                                                                                                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                          Block>(p)
                                                                                                                                     }),
                                                                                                                      context,
                                                                                                                      recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 fg_primitives::GrandpaApi<<Runtime as
                           self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn GrandpaApi_grandpa_pending_change_runtime_api_impl(&self,
                                                          at:
                                                              &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                   as
                                                                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                          context:
                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                          params:
                                                              Option<(&DigestFor<<Runtime
                                                                                 as
                                                                                 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>)>,
                                                          params_encoded:
                                                              Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<ScheduledChange<NumberFor<<Runtime
                                                                                                                                                                                                                   as
                                                                                                                                                                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 fg_primitives::runtime_decl_for_GrandpaApi::grandpa_pending_change_call_api_at(call_runtime_at,
                                                                                                                core_api,
                                                                                                                at,
                                                                                                                params_encoded,
                                                                                                                changes,
                                                                                                                initialized_block,
                                                                                                                params.map(|p|
                                                                                                                               {
                                                                                                                                   fg_primitives::runtime_decl_for_GrandpaApi::grandpa_pending_change_native_call_generator::<Runtime,
                                                                                                                                                                                                                              <Runtime
                                                                                                                                                                                                                              as
                                                                                                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                              Block>(p)
                                                                                                                               }),
                                                                                                                context,
                                                                                                                recorder)
                             })
    }
    fn GrandpaApi_grandpa_forced_change_runtime_api_impl(&self,
                                                         at:
                                                             &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                  as
                                                                                                                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                         context:
                                                             self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                         params:
                                                             Option<(&DigestFor<<Runtime
                                                                                as
                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>)>,
                                                         params_encoded:
                                                             Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<(NumberFor<<Runtime
                                                                                                                                                                                                    as
                                                                                                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                                                                                                                                                          ScheduledChange<NumberFor<<Runtime
                                                                                                                                                                                                                    as
                                                                                                                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>)>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 fg_primitives::runtime_decl_for_GrandpaApi::grandpa_forced_change_call_api_at(call_runtime_at,
                                                                                                               core_api,
                                                                                                               at,
                                                                                                               params_encoded,
                                                                                                               changes,
                                                                                                               initialized_block,
                                                                                                               params.map(|p|
                                                                                                                              {
                                                                                                                                  fg_primitives::runtime_decl_for_GrandpaApi::grandpa_forced_change_native_call_generator::<Runtime,
                                                                                                                                                                                                                            <Runtime
                                                                                                                                                                                                                            as
                                                                                                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                            Block>(p)
                                                                                                                              }),
                                                                                                               context,
                                                                                                               recorder)
                             })
    }
    fn GrandpaApi_grandpa_authorities_runtime_api_impl(&self,
                                                       at:
                                                           &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                as
                                                                                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                       context:
                                                           self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                       params: Option<()>,
                                                       params_encoded:
                                                           Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<(AuthorityId,
                                                                                                                                                                                       u64)>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 fg_primitives::runtime_decl_for_GrandpaApi::grandpa_authorities_call_api_at(call_runtime_at,
                                                                                                             core_api,
                                                                                                             at,
                                                                                                             params_encoded,
                                                                                                             changes,
                                                                                                             initialized_block,
                                                                                                             params.map(|p|
                                                                                                                            {
                                                                                                                                fg_primitives::runtime_decl_for_GrandpaApi::grandpa_authorities_native_call_generator::<Runtime,
                                                                                                                                                                                                                        <Runtime
                                                                                                                                                                                                                        as
                                                                                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                        Block>()
                                                                                                                            }),
                                                                                                             context,
                                                                                                             recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 consensus_aura::AuraApi<<Runtime as
                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn AuraApi_slot_duration_runtime_api_impl(&self,
                                              at:
                                                  &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                       as
                                                                                                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                              context:
                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                              params: Option<()>,
                                              params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<u64>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 consensus_aura::runtime_decl_for_AuraApi::slot_duration_call_api_at(call_runtime_at,
                                                                                                     core_api,
                                                                                                     at,
                                                                                                     params_encoded,
                                                                                                     changes,
                                                                                                     initialized_block,
                                                                                                     params.map(|p|
                                                                                                                    {
                                                                                                                        consensus_aura::runtime_decl_for_AuraApi::slot_duration_native_call_generator::<Runtime,
                                                                                                                                                                                                        <Runtime
                                                                                                                                                                                                        as
                                                                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                        Block>()
                                                                                                                    }),
                                                                                                     context,
                                                                                                     recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 consensus_authorities::AuthoritiesApi<<Runtime as
                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn AuthoritiesApi_authorities_runtime_api_impl(&self,
                                                   at:
                                                       &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                            as
                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                   context:
                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                   params: Option<()>,
                                                   params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<AuthorityIdFor<<Runtime
                                                                                                                                                                                                     as
                                                                                                                                                                                                     self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 consensus_authorities::runtime_decl_for_AuthoritiesApi::authorities_call_api_at(call_runtime_at,
                                                                                                                 core_api,
                                                                                                                 at,
                                                                                                                 params_encoded,
                                                                                                                 changes,
                                                                                                                 initialized_block,
                                                                                                                 params.map(|p|
                                                                                                                                {
                                                                                                                                    consensus_authorities::runtime_decl_for_AuthoritiesApi::authorities_native_call_generator::<Runtime,
                                                                                                                                                                                                                                <Runtime
                                                                                                                                                                                                                                as
                                                                                                                                                                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                Block>()
                                                                                                                                }),
                                                                                                                 context,
                                                                                                                 recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 runtime_api::xassets_api::XAssetsApi<<Runtime as
                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn XAssetsApi_valid_assets_runtime_api_impl(&self,
                                                at:
                                                    &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                         as
                                                                                                                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                context:
                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                params: Option<()>,
                                                params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<xassets::Token>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xassets_api::runtime_decl_for_XAssetsApi::valid_assets_call_api_at(call_runtime_at,
                                                                                                                 core_api,
                                                                                                                 at,
                                                                                                                 params_encoded,
                                                                                                                 changes,
                                                                                                                 initialized_block,
                                                                                                                 params.map(|p|
                                                                                                                                {
                                                                                                                                    runtime_api::xassets_api::runtime_decl_for_XAssetsApi::valid_assets_native_call_generator::<Runtime,
                                                                                                                                                                                                                                <Runtime
                                                                                                                                                                                                                                as
                                                                                                                                                                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                Block>()
                                                                                                                                }),
                                                                                                                 context,
                                                                                                                 recorder)
                             })
    }
    fn XAssetsApi_all_assets_runtime_api_impl(&self,
                                              at:
                                                  &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                       as
                                                                                                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                              context:
                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                              params: Option<()>,
                                              params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<(xassets::Asset,
                                                                                                                                                                                       bool)>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xassets_api::runtime_decl_for_XAssetsApi::all_assets_call_api_at(call_runtime_at,
                                                                                                               core_api,
                                                                                                               at,
                                                                                                               params_encoded,
                                                                                                               changes,
                                                                                                               initialized_block,
                                                                                                               params.map(|p|
                                                                                                                              {
                                                                                                                                  runtime_api::xassets_api::runtime_decl_for_XAssetsApi::all_assets_native_call_generator::<Runtime,
                                                                                                                                                                                                                            <Runtime
                                                                                                                                                                                                                            as
                                                                                                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                            Block>()
                                                                                                                              }),
                                                                                                               context,
                                                                                                               recorder)
                             })
    }
    fn XAssetsApi_valid_assets_of_runtime_api_impl(&self,
                                                   at:
                                                       &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                            as
                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                   context:
                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                   params:
                                                       Option<(AccountId)>,
                                                   params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<(xassets::Token,
                                                                                                                                                                                       BTreeMap<xassets::AssetType,
                                                                                                                                                                                                Balance>)>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xassets_api::runtime_decl_for_XAssetsApi::valid_assets_of_call_api_at(call_runtime_at,
                                                                                                                    core_api,
                                                                                                                    at,
                                                                                                                    params_encoded,
                                                                                                                    changes,
                                                                                                                    initialized_block,
                                                                                                                    params.map(|p|
                                                                                                                                   {
                                                                                                                                       runtime_api::xassets_api::runtime_decl_for_XAssetsApi::valid_assets_of_native_call_generator::<Runtime,
                                                                                                                                                                                                                                      <Runtime
                                                                                                                                                                                                                                      as
                                                                                                                                                                                                                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                      Block>(p)
                                                                                                                                   }),
                                                                                                                    context,
                                                                                                                    recorder)
                             })
    }
    fn XAssetsApi_withdrawal_list_of_runtime_api_impl(&self,
                                                      at:
                                                          &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                               as
                                                                                                                                               self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                      context:
                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                      params:
                                                          Option<(xassets::Chain)>,
                                                      params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<xrecords::RecordInfo<AccountId,
                                                                                                                                                                                                           Balance,
                                                                                                                                                                                                           BlockNumber,
                                                                                                                                                                                                           TimestampU64>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xassets_api::runtime_decl_for_XAssetsApi::withdrawal_list_of_call_api_at(call_runtime_at,
                                                                                                                       core_api,
                                                                                                                       at,
                                                                                                                       params_encoded,
                                                                                                                       changes,
                                                                                                                       initialized_block,
                                                                                                                       params.map(|p|
                                                                                                                                      {
                                                                                                                                          runtime_api::xassets_api::runtime_decl_for_XAssetsApi::withdrawal_list_of_native_call_generator::<Runtime,
                                                                                                                                                                                                                                            <Runtime
                                                                                                                                                                                                                                            as
                                                                                                                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                            Block>(p)
                                                                                                                                      }),
                                                                                                                       context,
                                                                                                                       recorder)
                             })
    }
    fn XAssetsApi_deposit_list_of_runtime_api_impl(&self,
                                                   at:
                                                       &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                            as
                                                                                                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                   context:
                                                       self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                   params:
                                                       Option<(xassets::Chain)>,
                                                   params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<xrecords::RecordInfo<AccountId,
                                                                                                                                                                                                           Balance,
                                                                                                                                                                                                           BlockNumber,
                                                                                                                                                                                                           TimestampU64>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xassets_api::runtime_decl_for_XAssetsApi::deposit_list_of_call_api_at(call_runtime_at,
                                                                                                                    core_api,
                                                                                                                    at,
                                                                                                                    params_encoded,
                                                                                                                    changes,
                                                                                                                    initialized_block,
                                                                                                                    params.map(|p|
                                                                                                                                   {
                                                                                                                                       runtime_api::xassets_api::runtime_decl_for_XAssetsApi::deposit_list_of_native_call_generator::<Runtime,
                                                                                                                                                                                                                                      <Runtime
                                                                                                                                                                                                                                      as
                                                                                                                                                                                                                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                      Block>(p)
                                                                                                                                   }),
                                                                                                                    context,
                                                                                                                    recorder)
                             })
    }
    fn XAssetsApi_verify_address_runtime_api_impl(&self,
                                                  at:
                                                      &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                           as
                                                                                                                                           self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                  context:
                                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                  params:
                                                      Option<(xassets::Token,
                                                              AddrStr,
                                                              xassets::Memo)>,
                                                  params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Result<(),
                                                                                                                                                                                         Vec<u8>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xassets_api::runtime_decl_for_XAssetsApi::verify_address_call_api_at(call_runtime_at,
                                                                                                                   core_api,
                                                                                                                   at,
                                                                                                                   params_encoded,
                                                                                                                   changes,
                                                                                                                   initialized_block,
                                                                                                                   params.map(|p|
                                                                                                                                  {
                                                                                                                                      runtime_api::xassets_api::runtime_decl_for_XAssetsApi::verify_address_native_call_generator::<Runtime,
                                                                                                                                                                                                                                    <Runtime
                                                                                                                                                                                                                                    as
                                                                                                                                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                    Block>(p.0,
                                                                                                                                                                                                                                           p.1,
                                                                                                                                                                                                                                           p.2)
                                                                                                                                  }),
                                                                                                                   context,
                                                                                                                   recorder)
                             })
    }
    fn XAssetsApi_withdrawal_limit_runtime_api_impl(&self,
                                                    at:
                                                        &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                             as
                                                                                                                                             self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                    context:
                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                    params:
                                                        Option<(xassets::Token)>,
                                                    params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<xprocess::WithdrawalLimit<Balance>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xassets_api::runtime_decl_for_XAssetsApi::withdrawal_limit_call_api_at(call_runtime_at,
                                                                                                                     core_api,
                                                                                                                     at,
                                                                                                                     params_encoded,
                                                                                                                     changes,
                                                                                                                     initialized_block,
                                                                                                                     params.map(|p|
                                                                                                                                    {
                                                                                                                                        runtime_api::xassets_api::runtime_decl_for_XAssetsApi::withdrawal_limit_native_call_generator::<Runtime,
                                                                                                                                                                                                                                        <Runtime
                                                                                                                                                                                                                                        as
                                                                                                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                        Block>(p)
                                                                                                                                    }),
                                                                                                                     context,
                                                                                                                     recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 runtime_api::xmining_api::XMiningApi<<Runtime as
                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn XMiningApi_jackpot_accountid_for_runtime_api_impl(&self,
                                                         at:
                                                             &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                  as
                                                                                                                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                         context:
                                                             self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                         params:
                                                             Option<(AccountId)>,
                                                         params_encoded:
                                                             Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<AccountId>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xmining_api::runtime_decl_for_XMiningApi::jackpot_accountid_for_call_api_at(call_runtime_at,
                                                                                                                          core_api,
                                                                                                                          at,
                                                                                                                          params_encoded,
                                                                                                                          changes,
                                                                                                                          initialized_block,
                                                                                                                          params.map(|p|
                                                                                                                                         {
                                                                                                                                             runtime_api::xmining_api::runtime_decl_for_XMiningApi::jackpot_accountid_for_native_call_generator::<Runtime,
                                                                                                                                                                                                                                                  <Runtime
                                                                                                                                                                                                                                                  as
                                                                                                                                                                                                                                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                                  Block>(p)
                                                                                                                                         }),
                                                                                                                          context,
                                                                                                                          recorder)
                             })
    }
    fn XMiningApi_multi_jackpot_accountid_for_runtime_api_impl(&self,
                                                               at:
                                                                   &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                        as
                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                               context:
                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                               params:
                                                                   Option<(Vec<AccountId>)>,
                                                               params_encoded:
                                                                   Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<AccountId>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xmining_api::runtime_decl_for_XMiningApi::multi_jackpot_accountid_for_call_api_at(call_runtime_at,
                                                                                                                                core_api,
                                                                                                                                at,
                                                                                                                                params_encoded,
                                                                                                                                changes,
                                                                                                                                initialized_block,
                                                                                                                                params.map(|p|
                                                                                                                                               {
                                                                                                                                                   runtime_api::xmining_api::runtime_decl_for_XMiningApi::multi_jackpot_accountid_for_native_call_generator::<Runtime,
                                                                                                                                                                                                                                                              <Runtime
                                                                                                                                                                                                                                                              as
                                                                                                                                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                                              Block>(p)
                                                                                                                                               }),
                                                                                                                                context,
                                                                                                                                recorder)
                             })
    }
    fn XMiningApi_token_jackpot_accountid_for_runtime_api_impl(&self,
                                                               at:
                                                                   &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                        as
                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                               context:
                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                               params:
                                                                   Option<(xassets::Token)>,
                                                               params_encoded:
                                                                   Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<AccountId>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xmining_api::runtime_decl_for_XMiningApi::token_jackpot_accountid_for_call_api_at(call_runtime_at,
                                                                                                                                core_api,
                                                                                                                                at,
                                                                                                                                params_encoded,
                                                                                                                                changes,
                                                                                                                                initialized_block,
                                                                                                                                params.map(|p|
                                                                                                                                               {
                                                                                                                                                   runtime_api::xmining_api::runtime_decl_for_XMiningApi::token_jackpot_accountid_for_native_call_generator::<Runtime,
                                                                                                                                                                                                                                                              <Runtime
                                                                                                                                                                                                                                                              as
                                                                                                                                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                                              Block>(p)
                                                                                                                                               }),
                                                                                                                                context,
                                                                                                                                recorder)
                             })
    }
    fn XMiningApi_multi_token_jackpot_accountid_for_runtime_api_impl(&self,
                                                                     at:
                                                                         &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                              as
                                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                                     context:
                                                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                                     params:
                                                                         Option<(Vec<xassets::Token>)>,
                                                                     params_encoded:
                                                                         Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<AccountId>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xmining_api::runtime_decl_for_XMiningApi::multi_token_jackpot_accountid_for_call_api_at(call_runtime_at,
                                                                                                                                      core_api,
                                                                                                                                      at,
                                                                                                                                      params_encoded,
                                                                                                                                      changes,
                                                                                                                                      initialized_block,
                                                                                                                                      params.map(|p|
                                                                                                                                                     {
                                                                                                                                                         runtime_api::xmining_api::runtime_decl_for_XMiningApi::multi_token_jackpot_accountid_for_native_call_generator::<Runtime,
                                                                                                                                                                                                                                                                          <Runtime
                                                                                                                                                                                                                                                                          as
                                                                                                                                                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                                                          Block>(p)
                                                                                                                                                     }),
                                                                                                                                      context,
                                                                                                                                      recorder)
                             })
    }
    fn XMiningApi_asset_power_runtime_api_impl(&self,
                                               at:
                                                   &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                        as
                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                               context:
                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                               params:
                                                   Option<(xassets::Token)>,
                                               params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<Balance>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xmining_api::runtime_decl_for_XMiningApi::asset_power_call_api_at(call_runtime_at,
                                                                                                                core_api,
                                                                                                                at,
                                                                                                                params_encoded,
                                                                                                                changes,
                                                                                                                initialized_block,
                                                                                                                params.map(|p|
                                                                                                                               {
                                                                                                                                   runtime_api::xmining_api::runtime_decl_for_XMiningApi::asset_power_native_call_generator::<Runtime,
                                                                                                                                                                                                                              <Runtime
                                                                                                                                                                                                                              as
                                                                                                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                              Block>(p)
                                                                                                                               }),
                                                                                                                context,
                                                                                                                recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 runtime_api::xspot_api::XSpotApi<<Runtime as
                                  self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn XSpotApi_aver_asset_price_runtime_api_impl(&self,
                                                  at:
                                                      &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                           as
                                                                                                                                           self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                  context:
                                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                  params:
                                                      Option<(xassets::Token)>,
                                                  params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<Balance>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xspot_api::runtime_decl_for_XSpotApi::aver_asset_price_call_api_at(call_runtime_at,
                                                                                                                 core_api,
                                                                                                                 at,
                                                                                                                 params_encoded,
                                                                                                                 changes,
                                                                                                                 initialized_block,
                                                                                                                 params.map(|p|
                                                                                                                                {
                                                                                                                                    runtime_api::xspot_api::runtime_decl_for_XSpotApi::aver_asset_price_native_call_generator::<Runtime,
                                                                                                                                                                                                                                <Runtime
                                                                                                                                                                                                                                as
                                                                                                                                                                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                Block>(p)
                                                                                                                                }),
                                                                                                                 context,
                                                                                                                 recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 runtime_api::xfee_api::XFeeApi<<Runtime as
                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn XFeeApi_transaction_fee_runtime_api_impl(&self,
                                                at:
                                                    &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                         as
                                                                                                                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                context:
                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                params:
                                                    Option<(Vec<u8>, u64)>,
                                                params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<u64>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xfee_api::runtime_decl_for_XFeeApi::transaction_fee_call_api_at(call_runtime_at,
                                                                                                              core_api,
                                                                                                              at,
                                                                                                              params_encoded,
                                                                                                              changes,
                                                                                                              initialized_block,
                                                                                                              params.map(|p|
                                                                                                                             {
                                                                                                                                 runtime_api::xfee_api::runtime_decl_for_XFeeApi::transaction_fee_native_call_generator::<Runtime,
                                                                                                                                                                                                                          <Runtime
                                                                                                                                                                                                                          as
                                                                                                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                          Block>(p.0,
                                                                                                                                                                                                                                 p.1)
                                                                                                                             }),
                                                                                                              context,
                                                                                                              recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 runtime_api::xsession_api::XSessionApi<<Runtime as
                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn XSessionApi_pubkeys_for_validator_name_runtime_api_impl(&self,
                                                               at:
                                                                   &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                        as
                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                               context:
                                                                   self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                               params:
                                                                   Option<(Vec<u8>)>,
                                                               params_encoded:
                                                                   Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<(AccountId,
                                                                                                                                                                                          Option<AuthorityId>)>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xsession_api::runtime_decl_for_XSessionApi::pubkeys_for_validator_name_call_api_at(call_runtime_at,
                                                                                                                                 core_api,
                                                                                                                                 at,
                                                                                                                                 params_encoded,
                                                                                                                                 changes,
                                                                                                                                 initialized_block,
                                                                                                                                 params.map(|p|
                                                                                                                                                {
                                                                                                                                                    runtime_api::xsession_api::runtime_decl_for_XSessionApi::pubkeys_for_validator_name_native_call_generator::<Runtime,
                                                                                                                                                                                                                                                                <Runtime
                                                                                                                                                                                                                                                                as
                                                                                                                                                                                                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                                                Block>(p)
                                                                                                                                                }),
                                                                                                                                 context,
                                                                                                                                 recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 runtime_api::xstaking_api::XStakingApi<<Runtime as
                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn XStakingApi_intention_set_runtime_api_impl(&self,
                                                  at:
                                                      &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                           as
                                                                                                                                           self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                  context:
                                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                  params: Option<()>,
                                                  params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Vec<AccountId>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xstaking_api::runtime_decl_for_XStakingApi::intention_set_call_api_at(call_runtime_at,
                                                                                                                    core_api,
                                                                                                                    at,
                                                                                                                    params_encoded,
                                                                                                                    changes,
                                                                                                                    initialized_block,
                                                                                                                    params.map(|p|
                                                                                                                                   {
                                                                                                                                       runtime_api::xstaking_api::runtime_decl_for_XStakingApi::intention_set_native_call_generator::<Runtime,
                                                                                                                                                                                                                                      <Runtime
                                                                                                                                                                                                                                      as
                                                                                                                                                                                                                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                      Block>()
                                                                                                                                   }),
                                                                                                                    context,
                                                                                                                    recorder)
                             })
    }
}
#[cfg(any(feature = "std", test))]
impl <RuntimeApiImplCall: self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::CallRuntimeAt<<Runtime
                                                                                                                    as
                                                                                                                    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock> +
      'static>
 runtime_api::xbridge_api::XBridgeApi<<Runtime as
                                      self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>
 for RuntimeApiImpl<RuntimeApiImplCall> {
    fn XBridgeApi_mock_new_trustees_runtime_api_impl(&self,
                                                     at:
                                                         &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                              as
                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                     context:
                                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                     params:
                                                         Option<(xassets::Chain,
                                                                 Vec<AccountId>)>,
                                                     params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Result<GenericAllSessionInfo<AccountId>,
                                                                                                                                                                                         Vec<u8>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::mock_new_trustees_call_api_at(call_runtime_at,
                                                                                                                      core_api,
                                                                                                                      at,
                                                                                                                      params_encoded,
                                                                                                                      changes,
                                                                                                                      initialized_block,
                                                                                                                      params.map(|p|
                                                                                                                                     {
                                                                                                                                         runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::mock_new_trustees_native_call_generator::<Runtime,
                                                                                                                                                                                                                                          <Runtime
                                                                                                                                                                                                                                          as
                                                                                                                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                          Block>(p.0,
                                                                                                                                                                                                                                                 p.1)
                                                                                                                                     }),
                                                                                                                      context,
                                                                                                                      recorder)
                             })
    }
    fn XBridgeApi_trustee_props_for_runtime_api_impl(&self,
                                                     at:
                                                         &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                              as
                                                                                                                                              self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                     context:
                                                         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                     params:
                                                         Option<(AccountId)>,
                                                     params_encoded: Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<BTreeMap<xassets::Chain,
                                                                                                                                                                                           GenericTrusteeIntentionProps>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::trustee_props_for_call_api_at(call_runtime_at,
                                                                                                                      core_api,
                                                                                                                      at,
                                                                                                                      params_encoded,
                                                                                                                      changes,
                                                                                                                      initialized_block,
                                                                                                                      params.map(|p|
                                                                                                                                     {
                                                                                                                                         runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::trustee_props_for_native_call_generator::<Runtime,
                                                                                                                                                                                                                                          <Runtime
                                                                                                                                                                                                                                          as
                                                                                                                                                                                                                                          self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                          Block>(p)
                                                                                                                                     }),
                                                                                                                      context,
                                                                                                                      recorder)
                             })
    }
    fn XBridgeApi_trustee_session_info_runtime_api_impl(&self,
                                                        at:
                                                            &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                 as
                                                                                                                                                 self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                        context:
                                                            self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                        params: Option<()>,
                                                        params_encoded:
                                                            Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<BTreeMap<xassets::Chain,
                                                                                                                                                                                           GenericAllSessionInfo<AccountId>>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::trustee_session_info_call_api_at(call_runtime_at,
                                                                                                                         core_api,
                                                                                                                         at,
                                                                                                                         params_encoded,
                                                                                                                         changes,
                                                                                                                         initialized_block,
                                                                                                                         params.map(|p|
                                                                                                                                        {
                                                                                                                                            runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::trustee_session_info_native_call_generator::<Runtime,
                                                                                                                                                                                                                                                <Runtime
                                                                                                                                                                                                                                                as
                                                                                                                                                                                                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                                Block>()
                                                                                                                                        }),
                                                                                                                         context,
                                                                                                                         recorder)
                             })
    }
    fn XBridgeApi_trustee_session_info_for_runtime_api_impl(&self,
                                                            at:
                                                                &self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::BlockId<<Runtime
                                                                                                                                                     as
                                                                                                                                                     self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock>,
                                                            context:
                                                                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ExecutionContext,
                                                            params:
                                                                Option<(xassets::Chain)>,
                                                            params_encoded:
                                                                Vec<u8>)
     ->
         self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::error::Result<self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::NativeOrEncoded<Option<(u32,
                                                                                                                                                                                          GenericAllSessionInfo<AccountId>)>>> {
        self.call_api_at(|call_runtime_at, core_api, changes,
                          initialized_block, recorder|
                             {
                                 runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::trustee_session_info_for_call_api_at(call_runtime_at,
                                                                                                                             core_api,
                                                                                                                             at,
                                                                                                                             params_encoded,
                                                                                                                             changes,
                                                                                                                             initialized_block,
                                                                                                                             params.map(|p|
                                                                                                                                            {
                                                                                                                                                runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::trustee_session_info_for_native_call_generator::<Runtime,
                                                                                                                                                                                                                                                        <Runtime
                                                                                                                                                                                                                                                        as
                                                                                                                                                                                                                                                        self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::GetNodeBlockType>::NodeBlock,
                                                                                                                                                                                                                                                        Block>(p)
                                                                                                                                            }),
                                                                                                                             context,
                                                                                                                             recorder)
                             })
    }
}
const RUNTIME_API_VERSIONS:
    self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::ApisVec =
    ::std::borrow::Cow::Borrowed(&[
        (
            client_api::runtime_decl_for_Core::ID,
            client_api::runtime_decl_for_Core::VERSION,
        ),
        (
            client_api::runtime_decl_for_Metadata::ID,
            client_api::runtime_decl_for_Metadata::VERSION,
        ),
        (
            block_builder_api::runtime_decl_for_BlockBuilder::ID,
            block_builder_api::runtime_decl_for_BlockBuilder::VERSION,
        ),
        (
            client_api::runtime_decl_for_TaggedTransactionQueue::ID,
            client_api::runtime_decl_for_TaggedTransactionQueue::VERSION,
        ),
        (
            offchain_primitives::runtime_decl_for_OffchainWorkerApi::ID,
            offchain_primitives::runtime_decl_for_OffchainWorkerApi::VERSION,
        ),
        (
            fg_primitives::runtime_decl_for_GrandpaApi::ID,
            fg_primitives::runtime_decl_for_GrandpaApi::VERSION,
        ),
        (
            consensus_aura::runtime_decl_for_AuraApi::ID,
            consensus_aura::runtime_decl_for_AuraApi::VERSION,
        ),
        (
            consensus_authorities::runtime_decl_for_AuthoritiesApi::ID,
            consensus_authorities::runtime_decl_for_AuthoritiesApi::VERSION,
        ),
        (
            runtime_api::xassets_api::runtime_decl_for_XAssetsApi::ID,
            runtime_api::xassets_api::runtime_decl_for_XAssetsApi::VERSION,
        ),
        (
            runtime_api::xmining_api::runtime_decl_for_XMiningApi::ID,
            runtime_api::xmining_api::runtime_decl_for_XMiningApi::VERSION,
        ),
        (
            runtime_api::xspot_api::runtime_decl_for_XSpotApi::ID,
            runtime_api::xspot_api::runtime_decl_for_XSpotApi::VERSION,
        ),
        (
            runtime_api::xfee_api::runtime_decl_for_XFeeApi::ID,
            runtime_api::xfee_api::runtime_decl_for_XFeeApi::VERSION,
        ),
        (
            runtime_api::xsession_api::runtime_decl_for_XSessionApi::ID,
            runtime_api::xsession_api::runtime_decl_for_XSessionApi::VERSION,
        ),
        (
            runtime_api::xstaking_api::runtime_decl_for_XStakingApi::ID,
            runtime_api::xstaking_api::runtime_decl_for_XStakingApi::VERSION,
        ),
        (
            runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::ID,
            runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::VERSION,
        ),
    ]);
pub mod api {
    use super::*;
    #[cfg(feature = "std")]
    pub fn dispatch(method: &str, mut data: &[u8]) -> Option<Vec<u8>> {
        match method {
            "Core_version" => Some({
                #[allow(deprecated)]
                let output = <Runtime as client_api::runtime_decl_for_Core::Core<Block>>::version();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "Core_execute_block" => Some({
                let block: Block =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"execute_block",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output =
                    <Runtime as client_api::runtime_decl_for_Core::Core<Block>>::execute_block(
                        block,
                    );
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "Core_initialize_block" => Some({
                let header: <Block as BlockT>::Header =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"initialize_block",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output =
                    <Runtime as client_api::runtime_decl_for_Core::Core<Block>>::initialize_block(
                        &header,
                    );
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "Core_authorities" => Some({
                #[allow(deprecated)]
                let output =
                    <Runtime as client_api::runtime_decl_for_Core::Core<Block>>::authorities();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "Metadata_metadata" => Some({
                #[allow(deprecated)]
                let output =
                    <Runtime as client_api::runtime_decl_for_Metadata::Metadata<Block>>::metadata();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "BlockBuilder_apply_extrinsic" => Some({
                let extrinsic: <Block as BlockT>::Extrinsic =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"apply_extrinsic",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output =
                    <Runtime as block_builder_api::runtime_decl_for_BlockBuilder::BlockBuilder<
                        Block,
                    >>::apply_extrinsic(extrinsic);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "BlockBuilder_finalize_block" => Some({
                #[allow(deprecated)]
                let output =
                    <Runtime as block_builder_api::runtime_decl_for_BlockBuilder::BlockBuilder<
                        Block,
                    >>::finalize_block();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "BlockBuilder_inherent_extrinsics" => Some({
                let data: InherentData =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"inherent_extrinsics",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output =
                    <Runtime as block_builder_api::runtime_decl_for_BlockBuilder::BlockBuilder<
                        Block,
                    >>::inherent_extrinsics(data);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "BlockBuilder_check_inherents" => Some({
                let block: Block =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"check_inherents",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                let data: InherentData =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"check_inherents",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output =
                    <Runtime as block_builder_api::runtime_decl_for_BlockBuilder::BlockBuilder<
                        Block,
                    >>::check_inherents(block, data);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "BlockBuilder_random_seed" => Some({
                #[allow(deprecated)]
                let output =
                    <Runtime as block_builder_api::runtime_decl_for_BlockBuilder::BlockBuilder<
                        Block,
                    >>::random_seed();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "TaggedTransactionQueue_validate_transaction" => Some({
                let tx: <Block as BlockT>::Extrinsic =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"validate_transaction",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             client_api::runtime_decl_for_TaggedTransactionQueue::TaggedTransactionQueue<Block>>::validate_transaction(tx);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "OffchainWorkerApi_offchain_worker" => Some({
                let number: NumberFor<Block> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"offchain_worker",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             offchain_primitives::runtime_decl_for_OffchainWorkerApi::OffchainWorkerApi<Block>>::offchain_worker(number);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "GrandpaApi_grandpa_pending_change" => Some({
                let digest: DigestFor<Block> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"grandpa_pending_change",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output = <Runtime as fg_primitives::runtime_decl_for_GrandpaApi::GrandpaApi<
                    Block,
                >>::grandpa_pending_change(&digest);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "GrandpaApi_grandpa_forced_change" => Some({
                let digest: DigestFor<Block> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"grandpa_forced_change",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output = <Runtime as fg_primitives::runtime_decl_for_GrandpaApi::GrandpaApi<
                    Block,
                >>::grandpa_forced_change(&digest);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "GrandpaApi_grandpa_authorities" => Some({
                #[allow(deprecated)]
                let output = <Runtime as fg_primitives::runtime_decl_for_GrandpaApi::GrandpaApi<
                    Block,
                >>::grandpa_authorities();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "AuraApi_slot_duration" => Some({
                #[allow(deprecated)]
                let output = <Runtime as consensus_aura::runtime_decl_for_AuraApi::AuraApi<
                    Block,
                >>::slot_duration();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "AuthoritiesApi_authorities" => Some({
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             consensus_authorities::runtime_decl_for_AuthoritiesApi::AuthoritiesApi<Block>>::authorities();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XAssetsApi_valid_assets" => Some({
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block>>::valid_assets();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XAssetsApi_all_assets" => Some({
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block>>::all_assets();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XAssetsApi_valid_assets_of" => Some({
                let who: AccountId =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"valid_assets_of",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block>>::valid_assets_of(who);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XAssetsApi_withdrawal_list_of" => Some({
                let chain: xassets::Chain =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"withdrawal_list_of",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block>>::withdrawal_list_of(chain);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XAssetsApi_deposit_list_of" => Some({
                let chain: xassets::Chain =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"deposit_list_of",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block>>::deposit_list_of(chain);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XAssetsApi_verify_address" => Some({
                let token: xassets::Token =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"verify_address",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                let addr: AddrStr =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"verify_address",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                let ext: xassets::Memo =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"verify_address",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block>>::verify_address(token,
                                                                                                                       addr,
                                                                                                                       ext);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XAssetsApi_withdrawal_limit" => Some({
                let token: xassets::Token =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"withdrawal_limit",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xassets_api::runtime_decl_for_XAssetsApi::XAssetsApi<Block>>::withdrawal_limit(token);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XMiningApi_jackpot_accountid_for" => Some({
                let who: AccountId =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"jackpot_accountid_for",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xmining_api::runtime_decl_for_XMiningApi::XMiningApi<Block>>::jackpot_accountid_for(who);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XMiningApi_multi_jackpot_accountid_for" => Some({
                let whos: Vec<AccountId> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"multi_jackpot_accountid_for",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xmining_api::runtime_decl_for_XMiningApi::XMiningApi<Block>>::multi_jackpot_accountid_for(whos);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XMiningApi_token_jackpot_accountid_for" => Some({
                let token: xassets::Token =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"token_jackpot_accountid_for",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xmining_api::runtime_decl_for_XMiningApi::XMiningApi<Block>>::token_jackpot_accountid_for(token);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XMiningApi_multi_token_jackpot_accountid_for" => Some({
                let tokens: Vec<xassets::Token> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"multi_token_jackpot_accountid_for",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xmining_api::runtime_decl_for_XMiningApi::XMiningApi<Block>>::multi_token_jackpot_accountid_for(tokens);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XMiningApi_asset_power" => Some({
                let token: xassets::Token =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"asset_power",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xmining_api::runtime_decl_for_XMiningApi::XMiningApi<Block>>::asset_power(token);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XSpotApi_aver_asset_price" => Some({
                let token: xassets::Token =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"aver_asset_price",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                let output =
                    <Runtime as runtime_api::xspot_api::runtime_decl_for_XSpotApi::XSpotApi<
                        Block,
                    >>::aver_asset_price(token);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XFeeApi_transaction_fee" => Some({
                let call_params: Vec<u8> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"transaction_fee",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                let encoded_len: u64 =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"transaction_fee",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xfee_api::runtime_decl_for_XFeeApi::XFeeApi<Block>>::transaction_fee(call_params,
                                                                                                               encoded_len);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XSessionApi_pubkeys_for_validator_name" => Some({
                let name: Vec<u8> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"pubkeys_for_validator_name",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xsession_api::runtime_decl_for_XSessionApi::XSessionApi<Block>>::pubkeys_for_validator_name(name);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XStakingApi_intention_set" => Some({
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xstaking_api::runtime_decl_for_XStakingApi::XStakingApi<Block>>::intention_set();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XBridgeApi_mock_new_trustees" => Some({
                let chain: xassets::Chain =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"mock_new_trustees",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                let candidates: Vec<AccountId> =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"mock_new_trustees",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::XBridgeApi<Block>>::mock_new_trustees(chain,
                                                                                                                          candidates);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XBridgeApi_trustee_props_for" => Some({
                let who: AccountId =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"trustee_props_for",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::XBridgeApi<Block>>::trustee_props_for(who);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XBridgeApi_trustee_session_info" => Some({
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::XBridgeApi<Block>>::trustee_session_info();
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            "XBridgeApi_trustee_session_info_for" => Some({
                let chain: xassets::Chain =
                         match self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Decode::decode(&mut data)
                             {
                             Some(input) => input,
                             None => {
                                 ::std::rt::begin_panic_fmt(&::std::fmt::Arguments::new_v1(&["Bad input data provided to "],
                                                                                           &match (&"trustee_session_info_for",)
                                                                                                {
                                                                                                (arg0,)
                                                                                                =>
                                                                                                [::std::fmt::ArgumentV1::new(arg0,
                                                                                                                             ::std::fmt::Display::fmt)],
                                                                                            }),
                                                            &("runtime/src/lib.rs",
                                                              269u32, 1u32))
                             }
                         };
                #[allow(deprecated)]
                     let output =
                         <Runtime as
                             runtime_api::xbridge_api::runtime_decl_for_XBridgeApi::XBridgeApi<Block>>::trustee_session_info_for(chain);
                self::sr_api_hidden_includes_IMPL_RUNTIME_APIS::sr_api_client::runtime_api::Encode::encode(&output)
            }),
            _ => None,
        }
    }
}
