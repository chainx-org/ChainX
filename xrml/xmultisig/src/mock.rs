// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use parity_codec::Decode;

use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup, Verify};
use primitives::BuildStorage;
use substrate_primitives::{Blake2Hasher, H256};
use support::{impl_outer_origin, Dispatchable};
use system::{ensure_root, ensure_signed};

impl_outer_origin! {
    pub enum Origin for Test {}
}

pub type Signature = substrate_primitives::ed25519::Signature;
pub type AccountId = <Signature as Verify>::Signer;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
pub struct Test2;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Debug)]
pub struct TestCall(pub bool);
pub struct TrusteeCall(TestCall);
impl From<TestCall> for TrusteeCall {
    fn from(call: TestCall) -> Self {
        TrusteeCall(call)
    }
}
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
impl LimitedCall<AccountId> for TrusteeCall {
    fn allow(&self) -> bool {
        !(self.0).0
    }

    fn exec(&self, _exerciser: &AccountId) -> Result {
        if !self.allow() {
            return Err("not allow");
        }
        Ok(())
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Debug)]
pub enum MyCall {
    #[allow(non_camel_case_types)]
    execute(AccountId, Box<Vec<u8>>),

    #[allow(non_camel_case_types)]
    confirm(AccountId, H256),

    #[allow(non_camel_case_types)]
    remove_multi_sig_for(AccountId, H256),

    #[allow(non_camel_case_types)]
    transition(Vec<(AccountId, bool)>, u32),

    #[allow(non_camel_case_types)]
    normal_call,

    #[allow(non_camel_case_types)]
    normal_call2,

    #[allow(non_camel_case_types)]
    root_call,
}
pub struct TrusteeCall2(MyCall);
impl From<MyCall> for TrusteeCall2 {
    fn from(call: MyCall) -> Self {
        TrusteeCall2(call)
    }
}
impl Dispatchable for MyCall {
    type Origin = Origin;
    type Trait = Test2;

    fn dispatch(self, origin: Self::Origin) -> Result {
        match self {
            MyCall::execute(multi_sig_addr, proposal) => {
                let call: MyCall = Decode::decode(&mut proposal.as_slice()).unwrap();
                MultiSig2::execute(origin, multi_sig_addr, Box::new(call))
            }
            MyCall::confirm(multi_sig_addr, multi_sig_id) => {
                MultiSig2::confirm(origin, multi_sig_addr, multi_sig_id)
            }
            MyCall::remove_multi_sig_for(multi_sig_addr, multi_sig_id) => {
                MultiSig2::remove_multi_sig_for(origin, multi_sig_addr, multi_sig_id)
            }
            MyCall::transition(owners, required_num) => {
                MultiSig2::transition(origin, owners, required_num)
            }
            MyCall::normal_call => ensure_signed(origin).map(|_| ()),
            MyCall::normal_call2 => ensure_signed(origin).map(|_| ()),
            MyCall::root_call => ensure_root(origin).map(|_| ()),
        }
    }
}
impl LimitedCall<AccountId> for TrusteeCall2 {
    fn allow(&self) -> bool {
        match &self.0 {
            MyCall::normal_call2 => true,
            _ => false,
        }
    }

    fn exec(&self, exerciser: &AccountId) -> Result {
        if !self.allow() {
            return Err("not allow");
        }
        let origin = system::RawOrigin::Signed(exerciser.clone()).into();
        self.0.clone().dispatch(origin)
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
    type Lookup = IdentityLookup<AccountId>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}
impl system::Trait for Test2 {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}
impl consensus::Trait for Test2 {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}
impl timestamp::Trait for Test2 {
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xsystem::Trait for Test {
    type ValidatorList = MockValidatorList;
    type Validator = MockValidator;
}
impl xsystem::Trait for Test2 {
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
    fn get_validator_name(_accountid: &AccountId) -> Option<Vec<u8>> {
        None
    }
}

impl xaccounts::Trait for Test {
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}
impl xaccounts::Trait for Test2 {
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

pub struct MockDeterminator;

impl xaccounts::IntentionJackpotAccountIdFor<AccountId> for MockDeterminator {
    fn accountid_for_unsafe(_origin: &AccountId) -> AccountId {
        unimplemented!()
    }

    fn accountid_for_safe(_origin: &AccountId) -> Option<AccountId> {
        unimplemented!()
    }
}

impl xsession::Trait for Test {
    type ConvertAccountIdToSessionKey = ();
    type OnSessionChange = ();
    type Event = ();
}

impl Trait for Test {
    type MultiSig = SimpleMultiSigIdFor<Test>;
    type GenesisMultiSig = ChainXGenesisMultisig<Test>;
    type Proposal = TestCall;
    type TrusteeCall = TrusteeCall;
    type Event = ();
}

impl Trait for Test2 {
    type MultiSig = SimpleMultiSigIdFor<Test2>;
    type GenesisMultiSig = ChainXGenesisMultisig<Test2>;
    type Proposal = MyCall;
    type TrusteeCall = TrusteeCall2;
    type Event = ();
}

pub type MultiSig = Module<Test>;
pub type MultiSig2 = Module<Test2>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    runtime_io::TestExternalities::new(t)
}

pub fn new_test_ext2() -> runtime_io::TestExternalities<Blake2Hasher> {
    let t = system::GenesisConfig::<Test2>::default()
        .build_storage()
        .unwrap()
        .0;

    runtime_io::TestExternalities::new(t)
}
