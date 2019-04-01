// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use parity_codec::Decode;

use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::BuildStorage;
use substrate_primitives::{Blake2Hasher, H256};
use support::{impl_outer_origin, Dispatchable};
use system::{ensure_root, ensure_signed};

impl_outer_origin! {
    pub enum Origin for Test {}
}

pub type AccountId = H256;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Debug)]
pub struct TestCall(pub bool);

impl Dispatchable for TestCall {
    type Origin = Origin;
    type Trait = ();

    fn dispatch(self, origin: Self::Origin) -> Result {
        if self.0 == true {
            ensure_root(origin)?;
        } else {
            ensure_signed(origin)?;
        }

        println!("call success");
        Err("call success")
    }
}

impl TrusteeCall<AccountId> for TestCall {
    fn allow(&self) -> bool {
        unimplemented!()
    }

    fn exec(&self, exerciser: &AccountId) -> Result {
        unimplemented!()
    }
}

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<H256>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = ();
    type OnNewAccount = ();
    type TransactionPayment = ();
    type TransferPayment = ();
    type DustRemoval = ();
    type Event = ();
}

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xsystem::Trait for Test {
    type ValidatorList = MockValidatorList;
    type Validator = MockValidator;
}

pub struct MockValidatorList;

impl xsystem::ValidatorList<AccountId> for MockValidatorList {
    fn validator_list() -> Vec<AccountId> {
        vec![]
    }
}

pub struct MockValidator;

impl xsystem::Validator<AccountId> for MockValidator {
    fn get_validator_by_name(_name: &[u8]) -> Option<AccountId> {
        unimplemented!()
    }
}

impl xaccounts::Trait for Test {
    type Event = ();
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

pub struct MockDeterminator;

impl xaccounts::IntentionJackpotAccountIdFor<AccountId> for MockDeterminator {
    fn accountid_for(_: &AccountId) -> AccountId {
        unimplemented!()
    }
}

impl xbitcoin::Trait for Test {
    type Event = ();
}

impl xstaking::Trait for Test {
    type Event = ();
    type OnRewardCalculation = ();
    type OnReward = ();
}

impl xsession::Trait for Test {
    type ConvertAccountIdToSessionKey = ();
    type OnSessionChange = ();
    type Event = ();
}

impl xfee_manager::Trait for Test {}

impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl Trait for Test {
    type MultiSig = SimpleMultiSigIdFor<Test>;
    type GenesisMultiSig = ChainXGenesisMultisig<Test>;
    type Proposal = TestCall;
    type Event = ();
}

//type Balances = balances::Module<Test>;
pub type MultiSig = Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    t.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![],
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            vesting: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );

    runtime_io::TestExternalities::new(t)
}
