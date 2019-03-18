// Copyright 2017-2018 Parity Technologies (UK) Ltd.
// Copyright 2018 Chainpool.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Executive: Handles all of the top-level stuff; essentially just executing blocks/extrinsics.

use crate::fee::CheckFee;
use parity_codec::{Codec, Encode};
use rstd::marker::PhantomData;
use rstd::prelude::*;
use rstd::result;
use runtime_io;
use runtime_primitives::traits::{
    self, Applyable, As, Block as BlockT, CheckEqual, Checkable, Digest, Hash, Header, NumberFor,
    OnFinalise, OnInitialise, One, Zero,
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
    /// Execute all extrinsics like when executing a `block`, but with dropping intial and final checks.
    fn execute_extrinsics_without_checks(
        block_number: NumberFor<Block>,
        extrinsics: Vec<Block::Extrinsic>,
    );
}

pub struct Executive<System, Block, Context, Payment, AllModules>(
    PhantomData<(System, Block, Context, Payment, AllModules)>,
);

impl<
    System: system::Trait + xfee_manager::Trait,
    Block: traits::Block<Header=System::Header, Hash=System::Hash>,
    Context: Default,
    Payment: MakePayment<System::AccountId>,
    AllModules: OnInitialise<System::BlockNumber> + OnFinalise<System::BlockNumber>,
> ExecuteBlock<Block> for Executive<System, Block, Context, Payment, AllModules> where
    Block::Extrinsic: Checkable<Context> + Codec,
    <Block::Extrinsic as Checkable<Context>>::Checked: Applyable<Index=System::Index, AccountId=System::AccountId> + Accelerable<Index=System::Index, AccountId=System::AccountId>,
    <<Block::Extrinsic as Checkable<Context>>::Checked as Applyable>::Call: Dispatchable + CheckFee,
    <<<Block::Extrinsic as Checkable<Context>>::Checked as Applyable>::Call as Dispatchable>::Origin: From<Option<System::AccountId>>
{
    fn execute_block(block: Block) {
        Self::execute_block(block);
    }

    fn execute_extrinsics_without_checks(block_number: NumberFor<Block>, extrinsics: Vec<Block::Extrinsic>) {
        Self::execute_extrinsics_without_checks(block_number, extrinsics);
    }
}

impl<
    System: system::Trait + xfee_manager::Trait,
    Block: traits::Block<Header=System::Header, Hash=System::Hash>,
    Context: Default,
    Payment: MakePayment<System::AccountId>,
    AllModules: OnInitialise<System::BlockNumber> + OnFinalise<System::BlockNumber>,
> Executive<System, Block, Context, Payment, AllModules> where
    Block::Extrinsic: Checkable<Context> + Codec,
    <Block::Extrinsic as Checkable<Context>>::Checked: Applyable<Index=System::Index, AccountId=System::AccountId> + Accelerable<Index=System::Index, AccountId=System::AccountId>,
    <<Block::Extrinsic as Checkable<Context>>::Checked as Applyable>::Call: Dispatchable + CheckFee,
    <<<Block::Extrinsic as Checkable<Context>>::Checked as Applyable>::Call as Dispatchable>::Origin: From<Option<System::AccountId>>
{
    /// Start the execution of a particular block.
    pub fn initialise_block(header: &System::Header) {
        Self::initialise_block_impl(header.number(), header.parent_hash(), header.extrinsics_root());
    }

    fn initialise_block_impl(block_number: &System::BlockNumber, parent_hash: &System::Hash, extrinsics_root: &System::Hash) {
        <system::Module<System>>::initialise(block_number, parent_hash, extrinsics_root);
        <AllModules as OnInitialise<System::BlockNumber>>::on_initialise(*block_number);
    }

    fn initial_checks(block: &Block) {
        let header = block.header();

        // check parent_hash is correct.
        let n = header.number().clone();
        assert!(
            n > System::BlockNumber::zero() && <system::Module<System>>::block_hash(n - System::BlockNumber::one()) == *header.parent_hash(),
            "Parent hash should be valid."
        );

        // check transaction trie root represents the transactions.
        let xts_root = extrinsics_root::<System::Hashing, _>(&block.extrinsics());
        header.extrinsics_root().check_equal(&xts_root);
        assert!(header.extrinsics_root() == &xts_root, "Transaction trie root must be valid.");
    }

    /// Actually execute all transitioning for `block`.
    pub fn execute_block(block: Block) {
        Self::initialise_block(block.header());

        // any initial checks
        Self::initial_checks(&block);

        // execute extrinsics
        let (header, extrinsics) = block.deconstruct();
        Self::execute_extrinsics_with_book_keeping(extrinsics, *header.number());

        // any final checks
        Self::final_checks(&header);
    }

    /// Execute all extrinsics like when executing a `block`, but with dropping intial and final checks.
    pub fn execute_extrinsics_without_checks(block_number: NumberFor<Block>, extrinsics: Vec<Block::Extrinsic>) {
        // Make the api happy, but maybe we should not set them at all.
        let parent_hash = <Block::Header as Header>::Hashing::hash(b"parent_hash");
        let extrinsics_root = <Block::Header as Header>::Hashing::hash(b"extrinsics_root");

        Self::initialise_block_impl(&block_number, &parent_hash, &extrinsics_root);

        // execute extrinsics
        Self::execute_extrinsics_with_book_keeping(extrinsics, block_number);
    }

    /// Execute given extrinsics and take care of post-extrinsics book-keeping
    fn execute_extrinsics_with_book_keeping(extrinsics: Vec<Block::Extrinsic>, block_number: NumberFor<Block>) {
        extrinsics.into_iter().for_each(Self::apply_extrinsic_no_note);

        // post-extrinsics book-keeping.
        <system::Module<System>>::note_finished_extrinsics();
        <AllModules as OnFinalise<System::BlockNumber>>::on_finalise(block_number);
    }

    /// Finalise the block - it is up the caller to ensure that all header fields are valid
    /// except state-root.
    pub fn finalise_block() -> System::Header {
        <system::Module<System>>::note_finished_extrinsics();
        <AllModules as OnFinalise<System::BlockNumber>>::on_finalise(<system::Module<System>>::block_number());

        // setup extrinsics
        <system::Module<System>>::derive_extrinsics();
        <system::Module<System>>::finalise()
    }

    /// Apply extrinsic outside of the block execution function.
    /// This doesn't attempt to validate anything regarding the block, but it builds a list of uxt
    /// hashes.
    pub fn apply_extrinsic(uxt: Block::Extrinsic) -> result::Result<ApplyOutcome, ApplyError> {
        let encoded = uxt.encode();
        let encoded_len = encoded.len();
        match Self::apply_extrinsic_with_len(uxt, encoded_len, Some(encoded)) {
            Ok(internal::ApplyOutcome::Success) => Ok(ApplyOutcome::Success),
            Ok(internal::ApplyOutcome::Fail(_)) => Ok(ApplyOutcome::Fail),
            Err(internal::ApplyError::CantPay) => Err(ApplyError::CantPay),
            Err(internal::ApplyError::BadSignature(_)) => Err(ApplyError::BadSignature),
            Err(internal::ApplyError::Stale) => Err(ApplyError::Stale),
            Err(internal::ApplyError::Future) => Err(ApplyError::Future),
            Err(internal::ApplyError::FullBlock) => Err(ApplyError::FullBlock),
            Err(internal::ApplyError::NotAllow) => Err(ApplyError::CantPay), // no other ApplyError for fee check failed
        }
    }

    /// Apply an extrinsic inside the block execution function.
    fn apply_extrinsic_no_note(uxt: Block::Extrinsic) {
        let l = uxt.encode().len();
        match Self::apply_extrinsic_with_len(uxt, l, None) {
            Ok(internal::ApplyOutcome::Success) => (),
            Ok(internal::ApplyOutcome::Fail(e)) => runtime_io::print(e),
            Err(internal::ApplyError::CantPay) => panic!("All extrinsics should have sender able to pay their fees"),
            Err(internal::ApplyError::BadSignature(_)) => panic!("All extrinsics should be properly signed"),
            Err(internal::ApplyError::Stale) | Err(internal::ApplyError::Future) => panic!("All extrinsics should have the correct nonce"),
            Err(internal::ApplyError::FullBlock) => panic!("Extrinsics should not exceed block limit"),
            Err(internal::ApplyError::NotAllow) => panic!("Extrinsics should not allow for this call"),
        }
    }

    /// Actually apply an extrinsic given its `encoded_len`; this doesn't note its hash.
    fn apply_extrinsic_with_len(uxt: Block::Extrinsic, encoded_len: usize, to_note: Option<Vec<u8>>) -> result::Result<internal::ApplyOutcome, internal::ApplyError> {
        // Verify the signature is good.
        let xt = uxt.check(&Default::default()).map_err(internal::ApplyError::BadSignature)?;

        // Check the size of the block if that extrinsic is applied.
        if <system::Module<System>>::all_extrinsics_len() + encoded_len as u32 > internal::MAX_TRANSACTIONS_SIZE {
            return Err(internal::ApplyError::FullBlock);
        }

        let mut signed_extrinsic = false;
        if let (Some(sender), Some(index)) = (xt.sender(), xt.index()) {
            // check index
            let expected_index = <system::Module<System>>::account_nonce(sender);
            if index != &expected_index {
                return Err(
                    if index < &expected_index { internal::ApplyError::Stale } else { internal::ApplyError::Future }
                ); }

            signed_extrinsic = true;
        }

        let acc = xt.acceleration();
        // decode parameters
        let (f, s) = xt.deconstruct();

        if signed_extrinsic {
            let acc = acc.unwrap();
            let switch = <xfee_manager::Module<System>>::switch();
            if let Some(fee_power) = f.check_fee(switch) {
                // pay any fees.
                Payment::make_payment(&s.clone().unwrap(), encoded_len, fee_power, acc.as_() as u32).map_err(|_| internal::ApplyError::CantPay)?;

                // AUDIT: Under no circumstances may this function panic from here onwards.

                // increment nonce in storage
                <system::Module<System>>::inc_account_nonce(&s.clone().unwrap());
            } else {
                return Err(internal::ApplyError::NotAllow);
            }
        }
        // make sure to `note_extrinsic` only after we know it's going to be executed
        // to prevent it from leaking in storage.
        if let Some(encoded) = to_note {
            <system::Module<System>>::note_extrinsic(encoded);
        }

        // and dispatch
        let r = f.dispatch(s.into());
        <system::Module<System>>::note_applied_extrinsic(&r, encoded_len as u32);

        r.map(|_| internal::ApplyOutcome::Success).or_else(|e| match e {
            runtime_primitives::BLOCK_FULL => Err(internal::ApplyError::FullBlock),
            e => Ok(internal::ApplyOutcome::Fail(e))
        })
    }

    fn final_checks(header: &System::Header) {
        // remove temporaries.
        let new_header = <system::Module<System>>::finalise();

        // check digest.
        assert_eq!(
            header.digest().logs().len(),
            new_header.digest().logs().len(),
            "Number of digest items must match that calculated."
        );
        let items_zip = header.digest().logs().iter().zip(new_header.digest().logs().iter());
        for (header_item, computed_item) in items_zip {
            header_item.check_equal(&computed_item);
            assert!(header_item == computed_item, "Digest item must match that calculated.");
        }

        // check storage root.
        let storage_root = System::Hashing::storage_root();
        header.state_root().check_equal(&storage_root);
        assert!(header.state_root() == &storage_root, "Storage root must match that calculated.");
    }

    /// Check a given transaction for validity. This doesn't execute any
    /// side-effects; it merely checks whether the transaction would panic if it were included or not.
    ///
    /// Changes made to the storage should be discarded.
    pub fn validate_transaction(uxt: Block::Extrinsic) -> TransactionValidity {
        // Note errors > 0 are from ApplyError
        const UNKNOWN_ERROR: i8 = -127;
        const MISSING_SENDER: i8 = -20;
        const INVALID_INDEX: i8 = -10;
        const ACC_ERROR: i8 = -30;
        const NOT_ALLOW: i8 = -1;

        let encoded_len = uxt.encode().len();

        let xt = match uxt.check(&Default::default()) {
            // Checks out. Carry on.
            Ok(xt) => xt,
            // An unknown account index implies that the transaction may yet become valid.
            Err("invalid account index") => return TransactionValidity::Unknown(INVALID_INDEX),
            // Technically a bad signature could also imply an out-of-date account index, but
            // that's more of an edge case.
            Err(runtime_primitives::BAD_SIGNATURE) => return TransactionValidity::Invalid(ApplyError::BadSignature as i8),
            Err(_) => return TransactionValidity::Invalid(UNKNOWN_ERROR),
        };

        // Acceleration can't be zero or empty.
        match xt.acceleration() {
            Some(acc) => {
                if acc.is_zero() {
                    return TransactionValidity::Invalid(ACC_ERROR);
                }
            }
            None => return TransactionValidity::Invalid(ACC_ERROR),
        }

        let valid = if let (Some(sender), Some(index), Some(acceleration)) = (xt.sender(), xt.index(), xt.acceleration()) {
            // check index
            let mut expected_index = <system::Module<System>>::account_nonce(sender);
            if index < &expected_index {
                return TransactionValidity::Invalid(ApplyError::Stale as i8);
            }
            if *index > expected_index + As::sa(256) {
                return TransactionValidity::Unknown(ApplyError::Future as i8);
            }

            let mut deps = Vec::new();
            while expected_index < *index {
                deps.push((sender, expected_index).encode());
                expected_index = expected_index + One::one();
            }

            TransactionValidity::Valid {
                priority: acceleration.as_() as TransactionPriority,
                requires: deps,
                provides: vec![(sender, *index).encode()],
                longevity: TransactionLongevity::max_value(),
            }
        } else {
            TransactionValidity::Invalid(if xt.sender().is_none() {
                MISSING_SENDER
            } else {
                INVALID_INDEX
            })
        };

        let acc = xt.acceleration().unwrap();
        let (f, s) = xt.deconstruct();
        let switch = <xfee_manager::Module<System>>::switch();
        if let Some(fee_power) = f.check_fee(switch) {
            if Payment::check_payment(&s.clone().unwrap(), encoded_len, fee_power, acc.as_() as u32).is_err() {
                return TransactionValidity::Invalid(ApplyError::CantPay as i8);
            } else {
                return valid;
            }
        } else {
            return TransactionValidity::Invalid(NOT_ALLOW as i8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fee::CheckFee;
    use balances::Call;
    use hex_literal::{hex, hex_impl};
    use runtime_io::with_externalities;
    use runtime_primitives::testing::{Block, Digest, DigestItem, Header};
    use runtime_primitives::traits::{
        self, Applyable, BlakeTwo256, Checkable, Header as HeaderT, IdentityLookup, Member,
    };
    use runtime_primitives::BuildStorage;
    use substrate_primitives::{Blake2Hasher, H256};
    use support::{impl_outer_event, impl_outer_origin, traits::Currency};
    use system;
    //use Acceleration;

    use crate::Runtime;
    use parity_codec::Decode;
    use serde::{Serialize, Serializer};
    use std::{fmt, fmt::Debug};

    /*impl_outer_origin! {
        pub enum Origin for Runtime {
        }
    }

    impl_outer_event! {
        pub enum MetaEvent for Runtime {
            balances<T>, xaccounts<T>,
        }
    }

    // Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
    #[derive(Clone, Eq, PartialEq)]
    pub struct Runtime;

    impl system::Trait for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = substrate_primitives::H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<u64>;
        type Header = Header;
        type Event = MetaEvent;
        type Log = DigestItem;
    }

    impl balances::Trait for Runtime {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = MetaEvent;
    }

    impl xfee_manager::Trait for Runtime {}

    /*impl xsystem::Trait for Runtime {
    }*/

    impl xaccounts::Trait for Runtime {
        type Event = MetaEvent;
    }

    impl CheckFee for Call<Runtime> {
        fn check_fee(&self) -> Option<u64> {
            // ret fee_power,     total_fee = base_fee * fee_power + byte_fee * bytes
            Some(1 as u64)
        }
    }*/

    // Snippets from sr_primitives::testing, fitting acceleration in.
    #[derive(PartialEq, Eq, Clone, Encode, Decode)]
    struct XTestXt<Call>(pub Option<u64>, pub u64, pub Call, pub u32);

    impl<Call> Serialize for XTestXt<Call>
    where
        XTestXt<Call>: Encode,
    {
        fn serialize<S>(&self, seq: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.using_encoded(|bytes| seq.serialize_bytes(bytes))
        }
    }

    impl<Call> Debug for XTestXt<Call> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "XTestXt({:?}, {:?}, {:?})", self.0, self.1, self.3)
        }
    }

    impl<Call: Codec + Sync + Send, Context> Checkable<Context> for XTestXt<Call> {
        type Checked = Self;
        fn check(self, _: &Context) -> Result<Self::Checked, &'static str> {
            Ok(self)
        }
    }

    impl<Call: Codec + Sync + Send> traits::Extrinsic for XTestXt<Call> {
        fn is_signed(&self) -> Option<bool> {
            None
        }
    }

    impl<Call> Applyable for XTestXt<Call>
    where
        Call: 'static + Sized + Send + Sync + Clone + Eq + Codec + Debug,
    {
        type AccountId = u64;
        type Index = u64;
        type Call = Call;
        fn sender(&self) -> Option<&u64> {
            self.0.as_ref()
        }
        fn index(&self) -> Option<&u64> {
            self.0.as_ref().map(|_| &self.1)
        }
        fn deconstruct(self) -> (Self::Call, Option<Self::AccountId>) {
            (self.2, self.0)
        }
    }

    impl<Call> xr_primitives::traits::Accelerable for XTestXt<Call>
    where
        Call: Member,
    {
        type Index = u64;
        type AccountId = u64;
        type Call = Call;
        type Acceleration = u32;

        fn acceleration(&self) -> Option<Self::Acceleration> {
            Some(self.3)
        }
    }

    type TestXt = XTestXt<Call<Runtime>>;

    /*type Executive = super::Executive<
        Runtime,
        Block<TestXt>,
        balances::ChainContext<Runtime>,
        fee_manager::Module<Runtime>,
        (),
    >;*/
    use crate::Executive;

    #[test]
    fn balance_transfer_dispatch_works() {
        let mut t = system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap()
            .0;
        t.extend(
            balances::GenesisConfig::<Runtime> {
                balances: vec![(1, 111)],
                transaction_base_fee: 10,
                transaction_byte_fee: 0,
                existential_deposit: 0,
                transfer_fee: 0,
                creation_fee: 0,
                reclaim_rebate: 0,
            }
            .build_storage()
            .unwrap()
            .0,
        );
        let xt = XTestXt(Some(1), 0, Call::transfer(2.into(), 69.into()), 1);
        let mut t = runtime_io::TestExternalities::<Blake2Hasher>::new(t);
        with_externalities(&mut t, || {
            Executive::initialise_block(&Header::new(
                1,
                H256::default(),
                H256::default(),
                [69u8; 32].into(),
                Digest::default(),
            ));
            Executive::apply_extrinsic(xt).unwrap();
            assert_eq!(<balances::Module<Runtime>>::total_balance(&1), 32);
            assert_eq!(<balances::Module<Runtime>>::total_balance(&2), 69);
        });
    }

    #[test]
    fn accelerate_balance_transfer_dispatch_works() {
        let mut t = system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap()
            .0;
        t.extend(
            balances::GenesisConfig::<Runtime> {
                balances: vec![(1, 111)],
                transaction_base_fee: 10,
                transaction_byte_fee: 0,
                existential_deposit: 0,
                transfer_fee: 0,
                creation_fee: 0,
                reclaim_rebate: 0,
            }
            .build_storage()
            .unwrap()
            .0,
        );
        let xt = XTestXt(Some(1), 0, Call::transfer(2.into(), 69.into()), 2);
        let mut t = runtime_io::TestExternalities::<Blake2Hasher>::new(t);
        with_externalities(&mut t, || {
            Executive::initialise_block(&Header::new(
                1,
                H256::default(),
                H256::default(),
                [69u8; 32].into(),
                Digest::default(),
            ));
            Executive::apply_extrinsic(xt).unwrap();
            assert_eq!(<balances::Module<Runtime>>::total_balance(&1), 22);
            assert_eq!(<balances::Module<Runtime>>::total_balance(&2), 69);
        });
    }

    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap()
            .0;
        t.extend(
            balances::GenesisConfig::<Runtime>::default()
                .build_storage()
                .unwrap()
                .0,
        );
        t.into()
    }

    #[test]
    fn block_import_works() {
        with_externalities(&mut new_test_ext(), || {
            Executive::execute_block(Block {
                header: Header {
                    parent_hash: [69u8; 32].into(),
                    number: 1,
                    state_root: hex!(
                        "d9e26179ed13b3df01e71ad0bf622d56f2066a63e04762a83c0ae9deeb4da1d0"
                    )
                    .into(),
                    extrinsics_root: hex!(
                        "03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
                    )
                    .into(),
                    digest: Digest { logs: vec![] },
                },
                extrinsics: vec![],
            });
        });
    }

    #[test]
    #[should_panic]
    fn block_import_of_bad_state_root_fails() {
        with_externalities(&mut new_test_ext(), || {
            Executive::execute_block(Block {
                header: Header {
                    parent_hash: [69u8; 32].into(),
                    number: 1,
                    state_root: [0u8; 32].into(),
                    extrinsics_root: hex!(
                        "03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
                    )
                    .into(),
                    digest: Digest { logs: vec![] },
                },
                extrinsics: vec![],
            });
        });
    }

    #[test]
    #[should_panic]
    fn block_import_of_bad_extrinsic_root_fails() {
        with_externalities(&mut new_test_ext(), || {
            Executive::execute_block(Block {
                header: Header {
                    parent_hash: [69u8; 32].into(),
                    number: 1,
                    state_root: hex!(
                        "d9e26179ed13b3df01e71ad0bf622d56f2066a63e04762a83c0ae9deeb4da1d0"
                    )
                    .into(),
                    extrinsics_root: [0u8; 32].into(),
                    digest: Digest { logs: vec![] },
                },
                extrinsics: vec![],
            });
        });
    }

    #[test]
    fn bad_extrinsic_not_inserted() {
        let mut t = new_test_ext();
        let xt = XTestXt(Some(1), 42, Call::transfer(33.into(), 69.into()), 1);
        with_externalities(&mut t, || {
            Executive::initialise_block(&Header::new(
                1,
                H256::default(),
                H256::default(),
                [69u8; 32].into(),
                Digest::default(),
            ));
            assert!(Executive::apply_extrinsic(xt).is_err());
            assert_eq!(<system::Module<Runtime>>::extrinsic_index(), Some(0));
        });
    }
}
